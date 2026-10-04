#![forbid(unsafe_code)]

pub mod live;
pub mod model;
pub mod ui;

/// The Notion surface for the signed-in Notion Desktop account, opened on the
/// last page the user viewed.
pub fn production_root() -> ui::SurfaceRoot {
    ui::SurfaceRoot::production(
        live::production_notion_bootstrap_api(model::NotionRouteSource::LastOpened),
        ui::NotionSurfaceServices::new(
            live::production_notion_remote_image_api(),
            live::production_notion_local_icon_file_api(),
        ),
    )
}
