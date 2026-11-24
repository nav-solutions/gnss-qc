//! Keplerian orbits from radio ephemerides (GPS, QZSS, Galileo, BeiDou).
//!
//! This is implemented here rather than relying on:
//! - rinex "nav" feature: it pins nalgebra/anise versions that are incompatible with gnss-rtk,
//! - gnss-rtk EphemerisSource: work in progress (the Kepler iteration does not converge).
use gnss_rs::prelude::{Constellation, SV};
use hifitime::{Epoch, TimeScale, Unit};

use rinex::navigation::Ephemeris;

/// Speed of light (m/s)
pub const SPEED_OF_LIGHT_M_S: f64 = 299_792_458.0;

/// Keplerian parameters of one radio ephemeris, ready for repeated evaluation.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QcKepler {
    pub sv: SV,
    /// Time of Clock
    pub toc: Epoch,
    /// Time of Ephemeris, in the constellation timescale
    pub toe: Epoch,
    /// Time of Ephemeris, seconds of week
    pub toe_sow: f64,
    pub sqrt_a: f64,
    pub e: f64,
    pub m0: f64,
    pub dn: f64,
    pub i0: f64,
    pub idot: f64,
    pub omega0: f64,
    pub omega: f64,
    pub omega_dot: f64,
    pub cuc: f64,
    pub cus: f64,
    pub cic: f64,
    pub cis: f64,
    pub crc: f64,
    pub crs: f64,
    /// Clock bias (s), drift (s/s), drift rate (s/s²)
    pub clock: (f64, f64, f64),
    /// Group delay of the primary signal (s): GPS/QZSS TGD, Galileo BGD(E1,E5a),
    /// BeiDou TGD1 (B1I)
    pub tgd_s: Option<f64>,
}

/// Satellite state obtained from Keplerian solving
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QcKeplerState {
    /// ECEF position (m)
    pub position_m: (f64, f64, f64),
    /// ECEF velocity (m/s)
    pub velocity_m_s: (f64, f64, f64),
    /// Eccentric anomaly (rad)
    pub eccentric_anomaly: f64,
}

impl QcKepler {
    /// Builds [QcKepler] from a radio [Ephemeris] published at `toc` by `sv`.
    /// Returns None for unsupported constellations (Glonass, SBAS..) or incomplete frames.
    pub fn from_ephemeris(sv: SV, toc: Epoch, eph: &Ephemeris) -> Option<Self> {
        match sv.constellation {
            Constellation::GPS
            | Constellation::QZSS
            | Constellation::Galileo
            | Constellation::BeiDou => {}
            _ => return None,
        }

        let f = |key: &str| eph.get_orbit_f64(key);

        let tgd_s = match sv.constellation {
            Constellation::Galileo => f("bgdE5aE1"),
            Constellation::BeiDou => f("tgd1b1b3"),
            _ => f("tgd"),
        };

        Some(Self {
            sv,
            toc,
            toe: eph.toe(sv)?,
            toe_sow: f("toe")?,
            sqrt_a: f("sqrta")?,
            e: f("e")?,
            m0: f("m0")?,
            dn: f("deltaN")?,
            i0: f("i0")?,
            idot: f("idot")?,
            omega0: f("omega0")?,
            omega: f("omega")?,
            omega_dot: f("omegaDot")?,
            cuc: f("cuc")?,
            cus: f("cus")?,
            cic: f("cic")?,
            cis: f("cis")?,
            crc: f("crc")?,
            crs: f("crs")?,
            clock: eph.sv_clock(),
            tgd_s,
        })
    }

    fn timescale(&self) -> TimeScale {
        match self.sv.constellation {
            Constellation::BeiDou => TimeScale::BDT,
            Constellation::Galileo => TimeScale::GST,
            _ => TimeScale::GPST,
        }
    }

    /// Earth gravitational constant (m³/s²)
    fn gm(&self) -> f64 {
        match self.sv.constellation {
            Constellation::GPS | Constellation::QZSS => 3.986005E14,
            _ => 3.986004418E14,
        }
    }

    /// Earth rotation rate (rad/s)
    fn omega_e(&self) -> f64 {
        match self.sv.constellation {
            Constellation::BeiDou => 7.292115E-5,
            _ => 7.2921151467E-5,
        }
    }

    /// BeiDou GEO satellites require a dedicated rotation
    fn is_beidou_geo(&self) -> bool {
        self.sv.constellation == Constellation::BeiDou && (self.sv.prn <= 5 || self.sv.prn >= 59)
    }

    /// Time from ToE (s)
    fn t_k(&self, t: Epoch) -> f64 {
        (t.to_time_scale(self.timescale()) - self.toe.to_time_scale(self.timescale())).to_seconds()
    }

    /// ECEF position (m) and eccentric anomaly at `t`
    fn position(&self, t: Epoch) -> ((f64, f64, f64), f64) {
        let t_k = self.t_k(t);
        let a = self.sqrt_a * self.sqrt_a;
        let n = (self.gm() / a.powi(3)).sqrt() + self.dn;
        let m_k = self.m0 + n * t_k;

        // Newton iterations
        let mut e_k = m_k;
        for _ in 0..30 {
            let delta = (e_k - self.e * e_k.sin() - m_k) / (1.0 - self.e * e_k.cos());
            e_k -= delta;
            if delta.abs() < 1.0E-14 {
                break;
            }
        }

        let (sin_e, cos_e) = e_k.sin_cos();
        let v_k = ((1.0 - self.e * self.e).sqrt() * sin_e).atan2(cos_e - self.e);
        let phi_k = v_k + self.omega;
        let (sin_2phi, cos_2phi) = (2.0 * phi_k).sin_cos();

        let u_k = phi_k + self.cus * sin_2phi + self.cuc * cos_2phi;
        let r_k = a * (1.0 - self.e * cos_e) + self.crs * sin_2phi + self.crc * cos_2phi;
        let i_k = self.i0 + self.idot * t_k + self.cis * sin_2phi + self.cic * cos_2phi;

        let (x_p, y_p) = (r_k * u_k.cos(), r_k * u_k.sin());
        let (sin_i, cos_i) = i_k.sin_cos();
        let omega_e = self.omega_e();

        if self.is_beidou_geo() {
            // BDS ICD: inertial orientation, then -5° tilt and earth rotation
            let omega_k = self.omega0 + self.omega_dot * t_k - omega_e * self.toe_sow;
            let (sin_o, cos_o) = omega_k.sin_cos();

            let x_g = x_p * cos_o - y_p * cos_i * sin_o;
            let y_g = x_p * sin_o + y_p * cos_i * cos_o;
            let z_g = y_p * sin_i;

            let (sin_x, cos_x) = (-5.0_f64).to_radians().sin_cos();
            let (sin_z, cos_z) = (omega_e * t_k).sin_cos();

            // Rx(-5°)
            let (x1, y1, z1) = (x_g, cos_x * y_g + sin_x * z_g, -sin_x * y_g + cos_x * z_g);

            // Rz(omega_e . t_k)
            let position = (cos_z * x1 + sin_z * y1, -sin_z * x1 + cos_z * y1, z1);
            (position, e_k)
        } else {
            let omega_k = self.omega0 + (self.omega_dot - omega_e) * t_k - omega_e * self.toe_sow;
            let (sin_o, cos_o) = omega_k.sin_cos();

            let position = (
                x_p * cos_o - y_p * cos_i * sin_o,
                x_p * sin_o + y_p * cos_i * cos_o,
                y_p * sin_i,
            );
            (position, e_k)
        }
    }

    /// Satellite state (ECEF position and velocity) at `t`
    pub fn state(&self, t: Epoch) -> QcKeplerState {
        let (position_m, eccentric_anomaly) = self.position(t);
        let (p_minus, _) = self.position(t - 0.5 * Unit::Second);
        let (p_plus, _) = self.position(t + 0.5 * Unit::Second);

        QcKeplerState {
            position_m,
            velocity_m_s: (
                p_plus.0 - p_minus.0,
                p_plus.1 - p_minus.1,
                p_plus.2 - p_minus.2,
            ),
            eccentric_anomaly,
        }
    }

    /// Satellite clock offset (s) at `t`, without relativistic correction
    pub fn clock_offset_s(&self, t: Epoch) -> f64 {
        let dt = (t.to_time_scale(self.timescale()) - self.toc.to_time_scale(self.timescale()))
            .to_seconds();
        let (a0, a1, a2) = self.clock;
        a0 + a1 * dt + a2 * dt * dt
    }

    /// Relativistic clock correction (s) for this eccentric anomaly
    pub fn relativistic_correction_s(&self, eccentric_anomaly: f64) -> f64 {
        let f = -2.0 * self.gm().sqrt() / SPEED_OF_LIGHT_M_S.powi(2);
        f * self.e * self.sqrt_a * eccentric_anomaly.sin()
    }
}

/// Relativistic clock correction (s) from position (m) and velocity (m/s): -2 r.v / c²
pub fn relativistic_correction_s(
    position_m: (f64, f64, f64),
    velocity_m_s: (f64, f64, f64),
) -> f64 {
    let r_v = position_m.0 * velocity_m_s.0
        + position_m.1 * velocity_m_s.1
        + position_m.2 * velocity_m_s.2;
    -2.0 * r_v / SPEED_OF_LIGHT_M_S.powi(2)
}
