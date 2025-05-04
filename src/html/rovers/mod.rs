use crate::{
    context::{QcContext, QcIndexing},
    report::selector::Selector,
};

use maud::{html, Markup, PreEscaped, Render};

use itertools::Itertools;
use std::collections::HashMap;

mod rover;
use rover::Report as RoverReport;

/// Rovers report (one for each)
pub struct Report {
    selector: Selector<QcIndexing>,
    rovers: HashMap<QcIndexing, RoverReport>,
}

impl Default for Report {
    fn default() -> Self {
        Self {
            selector: Selector::new("rovers", false),
            rovers: Default::default(),
        }
    }
}

impl Report {
    pub fn has_content(&self) -> bool {
        self.selector.has_content()
    }

    pub fn new(ctx: &QcContext) -> Self {
        let mut rovers = HashMap::new();
        let mut selector = Selector::new("rovers-selector", false);

        for rover in ctx.observations.keys() {
            selector.add(rover);
            rovers.insert(rover.clone(), RoverReport::new(ctx, rover));
        }

        Self { rovers, selector }
    }

    fn javascript(&self) -> &str {
        "
        const rover_sel = document.getElementById('rovers-selector');

        // rover listener
        rover_sel.addEventListener('change', (event) => {
            console.log('selected rover: ' + event.target.value);

            const rovers = document.getElementsByClassName('data rover');
            console.log('found: ' + rovers.length);

            if (event.target.value == 'All' || event.target.value == 'Both') {
                for (let i = 0;  i < rovers.length; i++) {
                    rovers[i].style.display = 'block';
                }
            } else {
                for (let i = 0;  i < rovers.length; i++) {
                    if (rovers[i].id == event.target.value) {
                        rovers[i].style.display = 'block';
                    } else {
                        rovers[i].style.display = 'none';
                    }
                }
            }
        });
        "
    }
}

impl Render for Report {
    fn render(&self) -> Markup {
        html! {
            p {
                (self.selector.render())
            }
            @ for (index, rover) in self.rovers.keys().sorted().enumerate() {
                @ if let Some(content) = self.rovers.get(&rover) {
                    @ if index == 0 {
                        section class="rover" id=(rover.to_string()) style="display: block" {
                            (content.render())
                        }
                    } @ else {
                        section class="rover" id=(rover.to_string()) style="display: none"  {
                            (content.render())
                        }
                    }
                }
            }

            script {
                (PreEscaped(self.javascript()))
            }
        }
    }
}
