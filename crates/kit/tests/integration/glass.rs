//! Glass on a modal's scrim blurs the page under the scrim: the dialog and sheet hosts take a
//! backdrop snapshot before the scrim paints, which Subroutine's GPUI fork adds (see its
//! `docs/upstream.md`), and the glass surface's backdrop filter reads it.
use crate::common;
use gpui_kit::component::{
    ActiveTheme as _, WindowExt as _,
    glass::{Glass, GlassStyled as _},
};
use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{
    AppContext as _, BackdropSource, Context, Hsla, IntoElement, Render, TestAppContext, Window,
    WindowHandle, div, prelude::*, px, size, white,
};

struct Page;

impl Render for Page {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().bg(white())
    }
}

fn open_page(cx: &mut TestAppContext) -> WindowHandle<gpui_kit::component::Root> {
    cx.update(|cx| {
        gpui_kit::init(cx);
        cx.set_reduce_motion(true);
    });
    let (handle, _) =
        common::open_window(cx, Some(size(px(800.), px(600.))), |_, cx| cx.new(|_| Page));
    handle
}

/// Every backdrop filter on screen reads a snapshot taken before the scrim, and paints after
/// the scrim.
fn assert_glass_reads_the_page_under_the_scrim(window: &mut Window, cx: &mut gpui_kit::App) {
    window.render_frame(cx);
    let scrim_color: Hsla = cx.theme().overlay;
    let scene = window.rendered_scene();
    let scrim = scene
        .quads
        .iter()
        .find(|quad| quad.background.as_solid() == Some(scrim_color))
        .expect("the scrim is painted");
    assert!(!scene.backdrop_filters.is_empty(), "the surface is glass");
    for filter in &scene.backdrop_filters {
        let BackdropSource::Snapshot(snapshot) = filter.source else {
            panic!("the glass blurs the live frame, scrim included");
        };
        let snapshot = scene.backdrop_snapshots[snapshot];
        assert!(
            snapshot.order < scrim.order,
            "the snapshot is taken before the scrim"
        );
        assert!(scrim.order < filter.order, "the glass is on the scrim");
    }
}

#[gpui_kit::test]
fn a_glass_dialog_blurs_the_page_under_its_scrim(cx: &mut TestAppContext) {
    let handle = open_page(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.open_dialog(cx, |dialog, _, cx| {
            dialog.title("Glass").glass(Glass::frosted(), cx)
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        assert_glass_reads_the_page_under_the_scrim(window, cx);
    })
    .unwrap();
}

#[gpui_kit::test]
fn a_glass_sheet_blurs_the_page_under_its_scrim(cx: &mut TestAppContext) {
    let handle = open_page(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.open_sheet(cx, |sheet, _, cx| {
            sheet.title("Glass").glass(Glass::frosted(), cx)
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        assert_glass_reads_the_page_under_the_scrim(window, cx);
    })
    .unwrap();
}
