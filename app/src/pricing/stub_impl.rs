use warpui::{Entity, SingletonEntity};

#[derive(Debug)]
pub struct PricingInfoModel;

impl PricingInfoModel {
    pub fn new() -> Self {
        Self
    }

    pub fn update_pricing_info(&mut self, _pricing_info: (), _ctx: &mut warpui::ModelContext<Self>) {}
}

impl Default for PricingInfoModel {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum PricingInfoModelEvent {
    PricingInfoUpdated,
}

impl Entity for PricingInfoModel {
    type Event = PricingInfoModelEvent;
}

impl SingletonEntity for PricingInfoModel {}
