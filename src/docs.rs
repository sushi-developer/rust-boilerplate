use utoipa::{Modify, OpenApi};

fn public_base_url() -> String {
    std::env::var("PUBLIC_BASE_URL").unwrap_or_else(|_| String::from("http://localhost:3000"))
}

struct ServersAddon;

impl Modify for ServersAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        openapi.servers = Some(vec![
            utoipa::openapi::ServerBuilder::new()
                .url(public_base_url())
                .build(),
        ]);
    }
}

#[derive(OpenApi)]
#[openapi(
    info(title = "Boilerplate", version = "0.1.0"),
    modifiers(&ServersAddon),
)]
pub struct ApiDoc;
