//! Glass on a modal's scrim blurs the page under the scrim: the dialog and sheet hosts take a
//! backdrop snapshot before the scrim paints, which Subroutine's GPUI fork adds (see its
//! `docs/upstream.md`), and the glass surface's backdrop filter reads it. Glass with an
//! elevation blurs the page from before its own shadow, which the fork draws under the surface
//! too.
use crate::common;
use gpui_kit::component::{
    ActiveTheme as _, WindowExt as _,
    glass::{Glass, GlassElevation, GlassStyled as _, GlassSurface},
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

struct GlassControl(GlassElevation);

impl Render for GlassControl {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().bg(white()).child(
            div().size(px(40.)).glass(
                Glass::frosted()
                    .surface(GlassSurface::Control)
                    .elevation(self.0),
                cx,
            ),
        )
    }
}

fn render_glass_control(
    cx: &mut TestAppContext,
    elevation: GlassElevation,
    check: impl FnOnce(&gpui_kit::Scene),
) {
    cx.update(|cx| gpui_kit::init(cx));
    let (handle, _) = common::open_window(cx, Some(size(px(200.), px(200.))), |_, cx| {
        cx.new(|_| GlassControl(elevation))
    });
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        check(window.rendered_scene());
    })
    .unwrap();
}

#[gpui_kit::test]
fn raised_glass_blurs_the_page_from_before_its_own_shadow(cx: &mut TestAppContext) {
    render_glass_control(cx, GlassElevation::Raised, |scene| {
        let [filter] = scene.backdrop_filters.as_slice() else {
            panic!("one glass surface");
        };
        let BackdropSource::Snapshot(snapshot) = filter.source else {
            panic!("the glass blurs the live frame, its own shadow included");
        };
        let snapshot = scene.backdrop_snapshots[snapshot];
        let drop_shadows = scene
            .shadows
            .iter()
            .filter(|shadow| shadow.inset == 0)
            .collect::<Vec<_>>();
        assert!(!drop_shadows.is_empty(), "the glass is raised");
        for shadow in drop_shadows {
            assert!(snapshot.order < shadow.order, "the snapshot is taken first");
            assert!(shadow.order < filter.order, "the shadow is under the glass");
        }
    });
}

#[gpui_kit::test]
fn flat_glass_blurs_the_live_frame(cx: &mut TestAppContext) {
    render_glass_control(cx, GlassElevation::Flat, |scene| {
        assert!(scene.backdrop_snapshots.is_empty());
        assert_eq!(
            scene
                .backdrop_filters
                .iter()
                .map(|filter| filter.source)
                .collect::<Vec<_>>(),
            [BackdropSource::Live]
        );
    });
}
