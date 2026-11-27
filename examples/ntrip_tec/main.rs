//! Real-time TEC maps of a network of reference stations, from NTRIP streams.
//!
//! Deploys one NTRIP client per station (RTCM 3: MSM observations, coordinates of the station,
//! radio ephemerides), estimates the TEC of every line of sight in real-time
//! ([QcObsTecProcessor]), grids the estimates into TEC maps (one map every `--interval`)
//! and formats them as a standard IONEX file, one file per UTC day, rewritten every time
//! a new map is complete. See `docs/ntrip_tec.md`.
//!
//! ```bash
//! cargo run --release --features ntrip,ionex --example ntrip_tec -- \
//!     ntrip://user:password@caster.centipede.fr:2101/STATION1 \
//!     ntrip://user:password@caster.centipede.fr:2101/STATION2 \
//!     ntrip://user:password@caster.centipede.fr:2101/STATION3 \
//!     ephemerides=ntrip://user:password@products.igs-ip.net:2101/BCEP00BKG0 \
//!     --interval "15 min" --output /tmp/tec
//! ```
//!
//! UNCALIBRATED: the differential code biases (DCB) of the satellites and of the receivers
//! are not corrected. Press Ctrl+C to stop (the IONEX file of the day remains valid),
//! or use `--duration`.
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    str::FromStr,
    time::{Duration as StdDuration, Instant},
};

use gnss_qc::prelude::*;

use env_logger::{Builder, Env, Target};
use log::{debug, info};

mod maps;

use maps::{parse_point, GridExtent, Obs2TecSetup, TecCollector, TecMaps};

const USAGE: &str = "usage: ntrip_tec [OPTIONS] STREAM [STREAM ..]

STREAM:
    ntrip://[user[:password]@]host[:port]/MOUNT                one station of the network
    ephemerides=ntrip://[user[:password]@]host[:port]/MOUNT    radio ephemerides only
    (ntrips:// for TLS, default port: 2101)

OPTIONS:
    --position MOUNT:X,Y,Z       ECEF coordinates of a station (m), when its stream does
                                 not deliver them (RTCM 1005/1006)
    -c, --constellations LIST    constellations, comma separated (default: GPS)
    --elevation-mask DEGREES     elevation mask (default: 15)
    --shell-altitude KM          altitude of the single layer (default: 450)
    --interval PERIOD            one map every PERIOD (default: \"15 min\")
    --grid EXTENT                network (default), world or LAT_MIN,LAT_MAX,LON_MIN,LON_MAX
    --dlat DEGREES, --dlon DEGREES   grid spacing (default: 2.5 x 5)
    --radius DEGREES             influence radius of a pierce point (default: largest spacing)
    --min-samples COUNT          minimal number of pierce points per node (default: 3)
    --no-rotation                Earth-fixed maps (default: Sun-fixed frame)
    --point NAME:LAT,LON         point of interest, printed map after map (repeat as needed)
    --output DIR                 IONEX files directory (default: .)
    --prefix NAME                IONEX file name prefix (default: NTRIPTEC)
    --duration PERIOD            stops after PERIOD (default: never, Ctrl+C)
    --reference-epoch EPOCH      approximate date of recorded streams (replay)";

/// Command line
struct Options {
    stations: Vec<QcNtripConfig>,
    ephemerides: Vec<QcNtripConfig>,
    constellations: Vec<Constellation>,
    setup: Obs2TecSetup,
    output: PathBuf,
    prefix: String,
    duration: Option<StdDuration>,
}

fn fail(message: &str) -> ! {
    eprintln!("{}\n\n{}", message, USAGE);
    std::process::exit(1);
}

fn parse<T: FromStr>(option: &str, value: Option<String>) -> T {
    let value = value.unwrap_or_else(|| fail(&format!("{}: missing value", option)));
    value
        .parse::<T>()
        .unwrap_or_else(|_| fail(&format!("{}: invalid value \"{}\"", option, value)))
}

fn parse_args() -> Options {
    let mut options = Options {
        stations: Vec::new(),
        ephemerides: Vec::new(),
        constellations: vec![Constellation::GPS],
        setup: Obs2TecSetup {
            interval: Duration::from_seconds(60.0 * 15.0),
            ..Default::default()
        },
        output: PathBuf::from("."),
        prefix: "NTRIPTEC".to_string(),
        duration: None,
    };

    let mut positions = Vec::<(String, (f64, f64, f64))>::new();
    let mut radius = None;
    let mut reference_epoch = None;

    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{}", USAGE);
                std::process::exit(0);
            }
            "--position" => {
                let value = args
                    .next()
                    .unwrap_or_else(|| fail("--position: missing value"));
                let parsed = value.split_once(':').and_then(|(mount, xyz)| {
                    let xyz = xyz
                        .split(',')
                        .map(|v| v.trim().parse::<f64>())
                        .collect::<Result<Vec<_>, _>>()
                        .ok()?;
                    match xyz.as_slice() {
                        [x, y, z] => Some((mount.to_string(), (*x, *y, *z))),
                        _ => None,
                    }
                });
                positions.push(parsed.unwrap_or_else(|| {
                    fail(&format!(
                        "--position: expecting MOUNT:X,Y,Z, got \"{}\"",
                        value
                    ))
                }));
            }
            "-c" | "--constellations" => {
                let value = args
                    .next()
                    .unwrap_or_else(|| fail("--constellations: missing value"));
                options.constellations = value
                    .split(',')
                    .map(|c| {
                        Constellation::from_str(c.trim())
                            .unwrap_or_else(|_| fail(&format!("invalid constellation \"{}\"", c)))
                    })
                    .collect();
            }
            "--elevation-mask" => {
                options.setup.elevation_mask_deg = parse(&arg, args.next());
            }
            "--shell-altitude" => {
                options.setup.shell_altitude_km = parse(&arg, args.next());
            }
            "--interval" => options.setup.interval = parse(&arg, args.next()),
            "--grid" => {
                let value = args.next().unwrap_or_else(|| fail("--grid: missing value"));
                options.setup.extent = GridExtent::from_str(&value).unwrap_or_else(|e| fail(&e));
            }
            "--dlat" => options.setup.dlat_deg = parse(&arg, args.next()),
            "--dlon" => options.setup.dlon_deg = parse(&arg, args.next()),
            "--radius" => radius = Some(parse::<f64>(&arg, args.next())),
            "--min-samples" => options.setup.min_samples = parse(&arg, args.next()),
            "--no-rotation" => options.setup.rotation = false,
            "--point" => {
                let value = args
                    .next()
                    .unwrap_or_else(|| fail("--point: missing value"));
                options
                    .setup
                    .points
                    .push(parse_point(&value).unwrap_or_else(|e| fail(&e)));
            }
            "--output" => options.output = PathBuf::from(parse::<String>(&arg, args.next())),
            "--prefix" => options.prefix = parse(&arg, args.next()),
            "--duration" => {
                let duration = parse::<Duration>(&arg, args.next());
                options.duration = Some(StdDuration::from_secs_f64(duration.to_seconds()));
            }
            "--reference-epoch" => reference_epoch = Some(parse::<Epoch>(&arg, args.next())),
            _ if arg.starts_with('-') => fail(&format!("unknown option \"{}\"", arg)),
            _ => {
                let (role, url) = match arg.split_once('=') {
                    Some(("ephemerides", url)) => (QcNtripRole::Ephemerides, url),
                    Some((role, _)) if !role.contains("://") => {
                        fail(&format!("invalid role \"{}\": expecting ephemerides", role))
                    }
                    _ => (QcNtripRole::Rover, arg.as_str()),
                };

                let stream = url
                    .parse::<QcNtripConfig>()
                    .unwrap_or_else(|e| fail(&format!("{}: {}", url, e)))
                    .with_role(role);

                if role == QcNtripRole::Ephemerides {
                    options.ephemerides.push(stream);
                } else {
                    options.stations.push(stream);
                }
            }
        }
    }

    if options.stations.is_empty() {
        fail("at least one station is required");
    }

    if !(options.setup.dlat_deg > 0.0 && options.setup.dlon_deg > 0.0)
        || options.setup.interval.to_seconds() <= 0.0
    {
        fail("invalid grid spacing or map interval");
    }

    options.setup.radius_deg = radius.unwrap_or(options.setup.dlat_deg.max(options.setup.dlon_deg));

    for (mount, position) in positions {
        let station = options
            .stations
            .iter_mut()
            .find(|s| s.mount == mount)
            .unwrap_or_else(|| fail(&format!("--position: unknown station \"{}\"", mount)));
        *station = station.clone().with_reference_position_ecef_m(position);
    }

    if let Some(epoch) = reference_epoch {
        for stream in options
            .stations
            .iter_mut()
            .chain(options.ephemerides.iter_mut())
        {
            *stream = stream.clone().with_reference_epoch(epoch);
        }
    }

    options
}

/// "15M", "01H", "30S": IONEX file names
fn period_code(seconds: f64) -> String {
    let seconds = seconds.round() as u64;
    if seconds.is_multiple_of(86400) {
        format!("{:02}D", seconds / 86400)
    } else if seconds.is_multiple_of(3600) {
        format!("{:02}H", seconds / 3600)
    } else if seconds.is_multiple_of(60) {
        format!("{:02}M", seconds / 60)
    } else {
        format!("{:02}S", seconds)
    }
}

/// Daily IONEX file name, IGS long name convention:
/// `PREFIX_YYYYDDD0000_01D_15M_GIM.INX`
fn file_name(prefix: &str, day: Epoch, interval: Duration) -> String {
    let (year, _, _, _, _, _, _) = day.to_gregorian_utc();
    let january = Epoch::from_gregorian_utc_at_midnight(year, 1, 1);
    let doy = ((day - january).to_seconds() / 86400.0).floor() as u32 + 1;
    format!(
        "{}_{:04}{:03}0000_01D_{}_GIM.INX",
        prefix,
        year,
        doy,
        period_code(interval.to_seconds())
    )
}

/// Real-time TEC maps: one IONEX file per UTC day
struct Publisher {
    setup: Obs2TecSetup,
    output: PathBuf,
    prefix: String,
    collector: TecCollector,
    /// UTC day of the IONEX file in progress
    day: Option<Epoch>,
    /// Latest complete map
    published: Option<Epoch>,
}

impl Publisher {
    fn half(&self) -> Duration {
        Duration::from_seconds(self.setup.interval.to_seconds() / 2.0)
    }

    /// Latest map whose pierce points are all known at `now`
    fn complete_map(&self, now: Epoch) -> Epoch {
        (now - self.half()).floor(self.setup.interval)
    }

    /// Consumes one result, publishes when a new map is complete
    fn consume(&mut self, item: &QcOutputItem) {
        self.collector.consume(item);

        let now = item.epoch.to_time_scale(TimeScale::UTC);

        if self.collector.samples.is_empty() {
            return;
        }

        let complete = self.complete_map(now);

        // first map: the one of the first pierce point
        let first = self.collector.samples[0].epoch.round(self.setup.interval);

        if complete >= first && self.published.is_none_or(|t| complete > t) {
            self.publish(complete, false);
        }
    }

    /// Writes the IONEX file of this day, maps up to `last` (included)
    fn write(&self, day: Epoch, last: Epoch) -> Option<TecMaps> {
        let end = (last + self.half()).min(day + Duration::from_days(1.0) + self.half());
        let maps = self.collector.grid(&self.setup, day - self.half(), end)?;
        let ionex = maps::ionex(&self.collector, &self.setup, &maps, "GNSS-Qc ntrip_tec")?;

        let path = self
            .output
            .join(file_name(&self.prefix, day, self.setup.interval));

        // the file is replaced atomically: it is always valid
        let tmp = path.with_extension("INX.tmp");
        match ionex.to_file(&tmp) {
            Ok(_) => {
                if let Err(e) = std::fs::rename(&tmp, &path) {
                    eprintln!("{}: {}", path.display(), e);
                }
            }
            Err(e) => eprintln!("{}: {:?}", path.display(), e),
        }

        Some(maps)
    }

    /// Publishes the maps up to `last`
    fn publish(&mut self, last: Epoch, end_of_streams: bool) {
        let day = last.floor(Duration::from_days(1.0));

        // a new day: last update of the previous day (exact leveling of the arcs that
        // ended since), then release its estimates
        if let Some(previous) = self.day {
            if day > previous {
                if let Some(maps) = self.write(previous, previous + Duration::from_days(1.0)) {
                    info!(
                        "{} completed: {} maps",
                        self.output
                            .join(file_name(&self.prefix, previous, self.setup.interval))
                            .display(),
                        maps.maps.len()
                    );
                }
                self.collector.prune(day - self.half());
            }
        }

        self.day = Some(day);
        self.published = Some(last);

        let last = if end_of_streams {
            day + Duration::from_days(1.0)
        } else {
            last
        };

        let Some(maps) = self.write(day, last) else {
            return;
        };

        if let Some((epoch, map)) = maps.maps.iter().next_back() {
            let mean =
                map.nodes.values().map(|n| n.tecu).sum::<f64>() / map.nodes.len().max(1) as f64;

            let points = map
                .points
                .iter()
                .map(|(name, node)| format!(", {}: {:.1} TECu", name, node.tecu))
                .collect::<String>();

            debug!(
                "{} map {}: {} receivers, {} pierce points, {}/{} nodes, mean {:.1} TECu{} (uncalibrated)",
                file_name(&self.prefix, day, self.setup.interval),
                epoch,
                map.receivers,
                map.samples,
                map.nodes.len(),
                maps.num_nodes(),
                mean,
                points,
            );
        }
    }
}

fn main() {
    let options = parse_args();

    Builder::from_env(Env::default().default_filter_or("debug"))
        .target(Target::Stderr)
        .format_timestamp_secs()
        .format_module_path(false)
        .init();

    std::fs::create_dir_all(&options.output).unwrap_or_else(|e| {
        fail(&format!("{}: {}", options.output.display(), e));
    });

    let cfg = QcObsTecConfig {
        elevation_mask_deg: options.setup.elevation_mask_deg,
        shell_altitude_km: options.setup.shell_altitude_km,
        earth_radius_km: options.setup.earth_radius_km,
        ..Default::default()
    }
    .with_constellations(&options.constellations);

    let mut builder = QcEngine::<QcOutput>::builder().with_processor(QcObsTecProcessor::new(cfg));

    for stream in options.stations.iter().chain(options.ephemerides.iter()) {
        info!("({}) {}", stream, stream.role);
        builder = builder.with_source(QcNtripSource::new(stream.clone()).unwrap());
    }

    let mut engine = builder.build().unwrap();

    info!(
        "TEC maps: one every {}, {} -> {}",
        options.setup.interval,
        options
            .constellations
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(", "),
        Path::new(&options.output).display(),
    );

    let mut publisher = Publisher {
        setup: options.setup.clone(),
        output: options.output.clone(),
        prefix: options.prefix.clone(),
        collector: TecCollector::default(),
        day: None,
        published: None,
    };

    let start = Instant::now();
    let mut last_report = Instant::now();
    let mut estimates = BTreeMap::<String, usize>::new();
    let mut latest = None;

    // event loop: the engine never blocks
    loop {
        if options.duration.is_some_and(|d| start.elapsed() > d) {
            info!("end of the session (--duration)");
            break;
        }

        match engine.poll_next() {
            Ok(QcPoll::Ready(item)) => {
                if let QcOutput::TecEstimate(estimate) = &item.payload {
                    *estimates.entry(estimate.receiver.to_string()).or_default() += 1;
                    latest = Some(item.epoch);
                }
                publisher.consume(&item);
            }
            Ok(QcPoll::Pending) => {
                if last_report.elapsed() > StdDuration::from_secs(30) {
                    debug!(
                        "{} TEC estimates per station: {:?}",
                        latest
                            .map(|t: Epoch| t.to_string())
                            .unwrap_or("waiting for data..".to_string()),
                        estimates
                    );
                    last_report = Instant::now();
                }
                std::thread::sleep(StdDuration::from_millis(20));
            }
            Ok(QcPoll::EndOfStream) => {
                info!("end of streams");
                break;
            }
            Err(e) => {
                eprintln!("{}", e);
                break;
            }
        }
    }

    // last update: every map of the day, partial maps included
    if let Some(t) = publisher.collector.samples.last().map(|s| s.epoch) {
        publisher.publish(t.floor(options.setup.interval), true);
    }

    info!("TEC estimates per station: {:?}", estimates);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn file_names() {
        let day = Epoch::from_str("2020-06-25T00:00:00 UTC").unwrap();
        assert_eq!(
            file_name("NTRIPTEC", day, Duration::from_seconds(60.0 * 15.0)),
            "NTRIPTEC_20201770000_01D_15M_GIM.INX"
        );
        assert_eq!(
            file_name("DNK", day, Duration::from_hours(1.0)),
            "DNK_20201770000_01D_01H_GIM.INX"
        );
        assert_eq!(period_code(30.0), "30S");
    }

    #[test]
    fn complete_maps() {
        let publisher = Publisher {
            setup: Obs2TecSetup {
                interval: Duration::from_seconds(60.0 * 15.0),
                ..Default::default()
            },
            output: PathBuf::from("."),
            prefix: "T".to_string(),
            collector: TecCollector::default(),
            day: None,
            published: None,
        };

        // 10:22:30: the window of the map 10:15 ([10:07:30, 10:22:30[) is complete
        let now = Epoch::from_str("2020-06-25T10:22:30 UTC").unwrap();
        assert_eq!(
            publisher.complete_map(now),
            Epoch::from_str("2020-06-25T10:15:00 UTC").unwrap()
        );
        let now = Epoch::from_str("2020-06-25T10:22:00 UTC").unwrap();
        assert_eq!(
            publisher.complete_map(now),
            Epoch::from_str("2020-06-25T10:00:00 UTC").unwrap()
        );
    }
}
