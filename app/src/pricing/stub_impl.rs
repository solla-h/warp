use warpui::{Entity, SingletonEntity};

/// Placeholder for the former GraphQL `StripeSubscriptionPlan` enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StripeSubscriptionPlan {
    Build,
    Other(String),
}

/// Wrapper exposing plan fields (e.g. `max_team_size`) that callers previously
/// accessed on the GraphQL struct. `plans()` returns an empty vec, so this is
/// only needed for type-checking.
#[derive(Debug, Clone)]
pub struct StripeSubscriptionPlanInfo {
    pub plan: StripeSubscriptionPlan,
    pub max_team_size: Option<i32>,
}

impl TryFrom<&crate::workspaces::workspace::BillingMetadata> for StripeSubscriptionPlan {
    type Error = ();
    fn try_from(
        _billing: &crate::workspaces::workspace::BillingMetadata,
    ) -> Result<Self, Self::Error> {
        Err(())
    }
}

/// Placeholder for plan pricing info.
#[derive(Debug, Clone)]
pub struct PlanPricing {
    pub monthly_plan_price_per_month_usd_cents: i32,
    pub yearly_plan_price_per_month_usd_cents: i32,
}

/// Placeholder for the former GraphQL `AddonCreditsOption` type.
#[derive(Debug, Clone)]
pub struct AddonCreditsOption {
    pub credits: i32,
    pub price_usd_cents: i32,
}

impl AddonCreditsOption {
    pub fn rate(&self) -> f64 {
        if self.credits == 0 {
            return 0.;
        }
        self.price_usd_cents as f64 / self.credits as f64
    }
}

#[derive(Debug)]
pub struct PricingInfoModel;

impl PricingInfoModel {
    pub fn new() -> Self {
        Self
    }

    pub fn plan_pricing(&self, _plan: &StripeSubscriptionPlan) -> Option<PlanPricing> {
        None
    }

    pub fn addon_credits_options(&self) -> Option<Vec<AddonCreditsOption>> {
        None
    }

    pub fn plans(&self) -> Vec<StripeSubscriptionPlanInfo> {
        Vec::new()
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
