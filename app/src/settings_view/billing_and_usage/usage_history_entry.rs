use warp_core::ui::appearance::Appearance;
use warpui::elements::{
    Border, Container, CornerRadius, CrossAxisAlignment, Empty, Flex, MainAxisAlignment,
    MainAxisSize, MouseStateHandle, ParentElement, Radius,
};
use warpui::{AppContext, Element};

pub struct UsageHistoryEntry {
    is_expanded: bool,
    mouse_state: Option<MouseStateHandle>,
    tooltip_mouse_state: MouseStateHandle,
}

impl UsageHistoryEntry {
    pub fn new(
        is_expanded: bool,
        mouse_state: Option<MouseStateHandle>,
        tooltip_mouse_state: MouseStateHandle,
    ) -> Self {
        Self {
            mouse_state,
            is_expanded,
            tooltip_mouse_state,
        }
    }

    pub fn render(&self, appearance: &Appearance, _app: &AppContext) -> Box<dyn Element> {
        let res = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(self.render_header(appearance));

        Container::new(res.finish())
            .with_border(Border::all(2.).with_border_fill(appearance.theme().surface_3()))
            .with_background(appearance.theme().surface_2())
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(8.)))
            .finish()
    }

    fn render_header(&self, appearance: &Appearance) -> Box<dyn Element> {
        self.render_loading_entry(appearance)
    }

    /// Render a placeholder entry for the loading state
    fn render_loading_entry(&self, appearance: &Appearance) -> Box<dyn Element> {
        let left_side = Flex::column()
            .with_child(self.render_empty_text_placeholder(360., 16., appearance))
            .with_child(self.render_empty_text_placeholder(160., 12., appearance))
            .with_spacing(4.)
            .with_cross_axis_alignment(CrossAxisAlignment::Start)
            .finish();

        Container::new(
            Flex::row()
                .with_child(left_side)
                .with_child(self.render_empty_text_placeholder(52., 16., appearance))
                .with_cross_axis_alignment(CrossAxisAlignment::Start)
                .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
                .with_main_axis_size(MainAxisSize::Max)
                .finish(),
        )
        .with_uniform_padding(12.)
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
        .with_background(appearance.theme().surface_2())
        .finish()
    }

    /// Renders an empty rectangle to represent loading text
    fn render_empty_text_placeholder(
        &self,
        width: f32,
        height: f32,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        Container::new(Empty::new().finish())
            .with_padding_left(width)
            .with_padding_top(height)
            .with_background(appearance.theme().surface_3())
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.)))
            .finish()
    }
}
