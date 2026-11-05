use crate::common;
use gpui_kit::component::{WindowExt, button::Button};
use gpui_kit::test::{TestAppContextExt, TestWindowExt};
use gpui_kit::{AppContext, Context, TestAppContext, Window, div, prelude::*, px, size};
use std::time::Duration;

struct SnappingSheet;

impl Render for SnappingSheet {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(
                Button::new("inspect")
                    .label("Inspect…")
                    .on_click(|_, window, cx| {
                        window.open_sheet(cx, |sheet, _, _| {
                            sheet
                                .title("Inspector")
                                .transition_duration(Duration::ZERO)
                                .child("Details")
                        });
                    }),
            )
    }
}

/// A sheet with a zero transition is in place on the frame it opens, where
/// the slide would start it off the window's edge.
#[gpui_kit::test]
async fn a_sheet_without_a_transition_opens_in_place(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, _) = common::open_window(cx, Some(size(px(800.), px(600.))), |_, cx| {
        cx.new(|_| SnappingSheet)
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click("inspect", cx);
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.try_find("sheet-content").is_some()
    })
    .await;
    cx.update_window(handle.into(), |_, window, _| {
        let sheet = window.find("sheet-content").bounds();
        assert_eq!(sheet.right(), window.viewport_size().width, "{sheet:?}");
    })
    .unwrap();
}
