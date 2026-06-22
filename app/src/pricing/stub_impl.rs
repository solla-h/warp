use warp_graphql::billing::{
    AddonCreditsOption, PlanPricing, PricingInfo, StripeSubscriptionPlan,
};
use warpui::{Entity, ModelContext, SingletonEntity};

#[derive(Debug)]
pub struct PricingInfoModel;

impl PricingInfoModel {
    pub fn new() -> Self {
        Self
    }

    pub fn update_pricing_info(&mut self, _pricing_info: PricingInfo, _ctx: &mut ModelContext<Self>) {
    }

    #[allow(dead_code)]
    pub fn plan_pricing(&self, _plan: &StripeSubscriptionPlan) -> Option<&PlanPricing> {
        None
    }

    pub fn plans(&self) -> &[PlanPricing] {
        &[]
    }

    #[allow(dead_code)]
    pub fn overage_cost_dollars(&self) -> Option<f64> {
        None
    }

    #[allow(dead_code)]
    pub fn monthly_plan_cost_dollars(&self, _plan: &StripeSubscriptionPlan) -> Option<f64> {
        None
    }

    pub fn addon_credits_options(&self) -> Option<&[AddonCreditsOption]> {
        None
    }
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
