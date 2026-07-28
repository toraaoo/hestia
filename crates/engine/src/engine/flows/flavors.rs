//! The flavor catalogue as a front-end sees it: what each distribution is, what
//! it takes, and what it needs on this machine that is not here.

use proto::minecraft::Flavor;

use super::Engine;
use crate::minecraft::unmet;

impl Engine {
    pub async fn server_flavors(&self) -> Vec<Flavor> {
        let mut flavors = self.minecraft.server_flavors();
        for flavor in &mut flavors {
            flavor.requires = unmet(self.minecraft.server_requires(&flavor.id)).await;
        }
        flavors
    }

    pub async fn instance_flavors(&self) -> Vec<Flavor> {
        let mut flavors = self.minecraft.instance_flavors();
        for flavor in &mut flavors {
            flavor.requires = unmet(self.minecraft.instance_requires(&flavor.id)).await;
        }
        flavors
    }
}
