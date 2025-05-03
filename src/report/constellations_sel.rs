use crate::prelude::{html, Constellation, Markup, Render, SV};

pub struct ConstellationsSelector {
    html_id: String,
    constellations: Vec<Constellation>,
}

impl ConstellationsSelector {
    pub fn new(html_id: &str) -> Self {
        Self {
            html_id: html_id.to_string(),
            constellations: Default::default(),
        }
    }

    pub fn has_content(&self) -> bool {
        !self.constellations.is_empty()
    }

    pub fn add_sv(&mut self, sv: &SV) {
        self.add_constellation(&sv.constellation);
    }

    pub fn add_constellation(&mut self, constell: &Constellation) {
        if !self.constellations.contains(constell) {
            self.constellations.push(*constell);
        }
    }
}

impl Render for ConstellationsSelector {
    fn render(&self) -> Markup {
        html! {
            div class="tabs" id=(self.html_id) {
                @ for (index, item) in self.constellations.iter().enumerate() {
                    @ if index == 0 {
                        div class="tab active" data-target=(item.to_string()) {
                            (item.to_string())
                        }
                    } @ else {
                        div class="tab" data-target=(item.to_string()) {
                            (item.to_string())
                        }
                    }
                }
            }
        }
    }
}
