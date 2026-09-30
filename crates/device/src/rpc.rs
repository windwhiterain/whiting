//! The http rpc surface, served with axum and serialized with facet.
//!
//! Every route takes and returns facet-serialized json. There are no hand written
//! conversion types for the domain: requests carry [`Value`], [`Schema`] and [`Delta`]
//! directly, so an agent edits exactly the documents the device stores.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::extract::{Path as RoutePath, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use definition::{Definition, DefinitionIdentifier, Delta, Schema, SchemaIdentifier, Value};
use facet::Facet;
use facet_axum::Json;
use graphics::GraphicsError;
use scene::CompileError;
use tokio::net::TcpListener;

use crate::render::RenderError;
use crate::state::{Device, DeviceError};

/// The shared device behind every route.
pub type Shared = Arc<std::sync::Mutex<Device>>;

/// Builds the router for a device.
pub fn router(device: Device) -> Router {
    let state: Shared = Arc::new(std::sync::Mutex::new(device));
    Router::new()
        .route("/health", get(health))
        .route("/renderer", get(renderer))
        .route("/schemas", get(list_schemas))
        .route("/schema/{name}", get(get_schema).put(put_schema))
        .route("/definitions", get(list_definitions))
        .route(
            "/definition/{name}",
            get(get_definition)
                .put(put_definition)
                .delete(delete_definition),
        )
        .route("/definition/{name}/delta", post(apply_delta))
        .route("/definition/{name}/undo", post(undo))
        .route("/definition/{name}/redo", post(redo))
        .route("/reload/{name}", post(reload))
        .route("/render/{name}", post(render))
        .route("/render/{name}/png", post(render_png))
        .route("/snapshot", post(snapshot))
        .route("/restore", post(restore))
        .with_state(state)
}

/// Binds a listener and serves until the process ends.
pub async fn serve(address: SocketAddr, device: Device) -> std::io::Result<()> {
    let listener = TcpListener::bind(address).await?;
    let served = axum::serve(listener, router(device));
    served.await
}

/// Reports that the server is up.
#[derive(Debug, Facet)]
pub struct Health {
    /// Always the literal `ok`, so a caller can distinguish a body from an error page.
    pub status: String,
}

/// Which renderer is active.
#[derive(Debug, Facet)]
pub struct RendererReport {
    /// Stable name of the renderer.
    pub name: String,
    /// Whether it can produce real pixels.
    pub available: bool,
}

/// A list of registered names.
#[derive(Debug, Facet)]
pub struct NameList {
    /// The names, in order.
    pub names: Vec<String>,
}

/// A schema, echoed back by the read route.
#[derive(Debug, Facet)]
pub struct SchemaReport {
    /// Name the schema is registered under.
    pub name: String,
    /// The schema document.
    pub schema: Schema,
}

/// A definition, echoed back by the read route.
#[derive(Debug, Facet)]
pub struct DefinitionReport {
    /// Name the definition is registered under.
    pub name: String,
    /// Name of the schema the value is verified against.
    pub schema: String,
    /// The current version.
    pub version: u64,
    /// The value itself.
    pub value: Value,
}

/// Registers a schema under the name in the route.
#[derive(Debug, Facet)]
pub struct PutSchema {
    /// The schema document.
    pub schema: Schema,
}

/// Replaces a definition outright.
#[derive(Debug, Facet)]
pub struct PutDefinition {
    /// Name of the schema the value is verified against.
    pub schema: String,
    /// The value itself.
    pub value: Value,
}

/// A delta submission.
#[derive(Debug, Facet)]
pub struct SubmitDelta {
    /// The sparse changes to apply.
    pub delta: Delta,
}

/// What a mutation did.
#[derive(Debug, Facet)]
pub struct MutationReport {
    /// Name of the definition that changed.
    pub name: String,
    /// Its version after the change.
    pub version: u64,
    /// How many operations were applied.
    pub operations: usize,
}

/// A reload outcome.
#[derive(Debug, Facet)]
pub struct ReloadResponse {
    /// The graphics output version after the reload.
    pub graphics_version: u64,
    /// The instance output version after the reload.
    pub instance_version: u64,
    /// Whether the graphics output was created rather than advanced.
    pub graphics_created: bool,
    /// Whether the instance output was created rather than advanced.
    pub instance_created: bool,
    /// How many operations the emitted graphics delta carried.
    pub graphics_operations: usize,
    /// How many operations the emitted instance delta carried.
    pub instance_operations: usize,
    /// Whether the cross-check ran.
    pub cross_checked: bool,
}

/// A rendered frame, returned as raw pixels.
#[derive(Debug, Facet)]
pub struct RenderResponse {
    /// Name of the renderer that produced the frame.
    pub renderer: String,
    /// Whether that renderer can produce real pixels.
    pub available: bool,
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Red, green, blue and alpha bytes, row major.
    pub rgba: Vec<u8>,
}

/// A snapshot or restore target path.
#[derive(Debug, Facet)]
pub struct SnapshotRequest {
    /// Filesystem path to write or read.
    pub path: String,
}

async fn health() -> Json<Health> {
    Json(Health {
        status: "ok".to_owned(),
    })
}

async fn renderer(State(device): State<Shared>) -> Json<RendererReport> {
    let device = device.lock().expect("the device lock is not poisoned");
    Json(RendererReport {
        name: device.renderer().name().to_owned(),
        available: device.renderer().available(),
    })
}

async fn list_schemas(State(device): State<Shared>) -> Json<NameList> {
    let device = device.lock().expect("the device lock is not poisoned");
    Json(NameList {
        names: device
            .registry()
            .schema_identifiers()
            .into_iter()
            .map(SchemaIdentifier::into_string)
            .collect(),
    })
}

async fn get_schema(
    State(device): State<Shared>,
    RoutePath(name): RoutePath<String>,
) -> Result<Json<SchemaReport>, ApiError> {
    let device = device.lock().expect("the device lock is not poisoned");
    let identifier = SchemaIdentifier::new(name);
    let schema = device.registry().schema(&identifier)?;
    Ok(Json(SchemaReport {
        name: identifier.into_string(),
        schema: schema.clone(),
    }))
}

async fn put_schema(
    State(device): State<Shared>,
    RoutePath(name): RoutePath<String>,
    Json(body): Json<PutSchema>,
) -> Result<Json<SchemaReport>, ApiError> {
    let mut device = device.lock().expect("the device lock is not poisoned");
    let identifier = SchemaIdentifier::new(name);
    device
        .registry_mut()
        .put_schema(identifier.clone(), body.schema.clone());
    device.forget_resolved();
    Ok(Json(SchemaReport {
        name: identifier.into_string(),
        schema: body.schema,
    }))
}

async fn list_definitions(State(device): State<Shared>) -> Json<NameList> {
    let device = device.lock().expect("the device lock is not poisoned");
    Json(NameList {
        names: device
            .registry()
            .definition_identifiers()
            .into_iter()
            .map(DefinitionIdentifier::into_string)
            .collect(),
    })
}

async fn get_definition(
    State(device): State<Shared>,
    RoutePath(name): RoutePath<String>,
) -> Result<Json<DefinitionReport>, ApiError> {
    let device = device.lock().expect("the device lock is not poisoned");
    let identifier = DefinitionIdentifier::new(name);
    let definition = device.registry().definition(&identifier)?;
    Ok(Json(report(&identifier, definition)))
}

async fn put_definition(
    State(device): State<Shared>,
    RoutePath(name): RoutePath<String>,
    Json(body): Json<PutDefinition>,
) -> Result<Json<DefinitionReport>, ApiError> {
    let mut device = device.lock().expect("the device lock is not poisoned");
    let identifier = DefinitionIdentifier::new(name);
    let definition = Definition::new(
        identifier.clone(),
        SchemaIdentifier::new(body.schema),
        body.value,
    );
    // A definition is only accepted once it satisfies the schema it names, so a bad
    // document is rejected at the boundary rather than at render time.
    definition.validate(device.registry())?;
    if device.registry().definition(&identifier).is_ok() {
        let _ = device.registry_mut().remove_definition(&identifier);
    }
    device.registry_mut().insert_definition(definition)?;
    device.forget_resolved();
    let stored = device.registry().definition(&identifier)?;
    Ok(Json(report(&identifier, stored)))
}

async fn delete_definition(
    State(device): State<Shared>,
    RoutePath(name): RoutePath<String>,
) -> Result<StatusCode, ApiError> {
    let mut device = device.lock().expect("the device lock is not poisoned");
    let identifier = DefinitionIdentifier::new(name);
    device.registry_mut().remove_definition(&identifier)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn apply_delta(
    State(device): State<Shared>,
    RoutePath(name): RoutePath<String>,
    Json(body): Json<SubmitDelta>,
) -> Result<Json<MutationReport>, ApiError> {
    let mut device = device.lock().expect("the device lock is not poisoned");
    let identifier = DefinitionIdentifier::new(name);
    let operations = body.delta.operations.len();
    device.registry_mut().apply(&body.delta)?;
    device.forget_resolved();
    Ok(Json(MutationReport {
        version: device.registry().definition(&identifier)?.version.get(),
        name: identifier.into_string(),
        operations,
    }))
}

async fn undo(
    State(device): State<Shared>,
    RoutePath(name): RoutePath<String>,
) -> Result<Json<MutationReport>, ApiError> {
    let mut device = device.lock().expect("the device lock is not poisoned");
    let identifier = DefinitionIdentifier::new(name);
    let delta = device.registry_mut().undo(&identifier)?;
    device.forget_resolved();
    Ok(Json(MutationReport {
        version: device.registry().definition(&identifier)?.version.get(),
        name: identifier.into_string(),
        operations: delta.operations.len(),
    }))
}

async fn redo(
    State(device): State<Shared>,
    RoutePath(name): RoutePath<String>,
) -> Result<Json<MutationReport>, ApiError> {
    let mut device = device.lock().expect("the device lock is not poisoned");
    let identifier = DefinitionIdentifier::new(name);
    let delta = device.registry_mut().redo(&identifier)?;
    device.forget_resolved();
    Ok(Json(MutationReport {
        version: device.registry().definition(&identifier)?.version.get(),
        name: identifier.into_string(),
        operations: delta.operations.len(),
    }))
}

async fn reload(
    State(device): State<Shared>,
    RoutePath(name): RoutePath<String>,
) -> Result<Json<ReloadResponse>, ApiError> {
    let mut device = device.lock().expect("the device lock is not poisoned");
    let report = device.reload(&DefinitionIdentifier::new(name))?;
    device.forget_resolved();
    Ok(Json(ReloadResponse {
        graphics_version: report.graphics_version.get(),
        instance_version: report.instance_version.get(),
        graphics_created: report.graphics_created,
        instance_created: report.instance_created,
        graphics_operations: report.graphics_operations,
        instance_operations: report.instance_operations,
        cross_checked: report.cross_checked,
    }))
}

async fn render(
    State(device): State<Shared>,
    RoutePath(name): RoutePath<String>,
) -> Result<Json<RenderResponse>, ApiError> {
    let device = device.lock().expect("the device lock is not poisoned");
    let identifier = DefinitionIdentifier::new(name);
    let rendered = device.render(&identifier)?;
    Ok(Json(RenderResponse {
        renderer: rendered.renderer.to_owned(),
        available: device.renderer().available(),
        width: rendered.frame.width,
        height: rendered.frame.height,
        rgba: rendered.frame.rgba,
    }))
}

#[cfg(feature = "png")]
async fn render_png(
    State(device): State<Shared>,
    RoutePath(name): RoutePath<String>,
) -> Result<Response, ApiError> {
    let device = device.lock().expect("the device lock is not poisoned");
    let rendered = device.render(&DefinitionIdentifier::new(name))?;
    let bytes = rendered.to_png()?;
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "image/png")],
        bytes,
    )
        .into_response())
}

#[cfg(not(feature = "png"))]
async fn render_png(
    State(_device): State<Shared>,
    RoutePath(_name): RoutePath<String>,
) -> Result<Response, ApiError> {
    Err(ApiError::Device(DeviceError::Body(
        "png rendering needs the `png` feature".to_owned(),
    )))
}

async fn snapshot(
    State(device): State<Shared>,
    Json(body): Json<SnapshotRequest>,
) -> Result<Json<MutationReport>, ApiError> {
    let device = device.lock().expect("the device lock is not poisoned");
    let path = std::path::PathBuf::from(&body.path);
    device.snapshot(&path)?;
    Ok(Json(MutationReport {
        name: body.path,
        version: 0,
        operations: 0,
    }))
}

async fn restore(
    State(device): State<Shared>,
    Json(body): Json<SnapshotRequest>,
) -> Result<Json<MutationReport>, ApiError> {
    let mut device = device.lock().expect("the device lock is not poisoned");
    let path = std::path::PathBuf::from(&body.path);
    device.restore(&path)?;
    device.forget_resolved();
    Ok(Json(MutationReport {
        name: body.path,
        version: 0,
        operations: 0,
    }))
}

fn report(identifier: &DefinitionIdentifier, definition: &Definition) -> DefinitionReport {
    DefinitionReport {
        name: identifier.as_str().to_owned(),
        schema: definition.schema_identifier.as_str().to_owned(),
        version: definition.version.get(),
        value: definition.value.clone(),
    }
}

/// A failed request, rendered as a status code and a message.
pub struct ApiError(DeviceError);

impl From<DeviceError> for ApiError {
    fn from(error: DeviceError) -> Self {
        Self(error)
    }
}

impl From<definition::ValidationError> for ApiError {
    fn from(error: definition::ValidationError) -> Self {
        Self(DeviceError::Validation(error))
    }
}

impl From<definition::RegistryError> for ApiError {
    fn from(error: definition::RegistryError) -> Self {
        Self(DeviceError::Registry(error))
    }
}

impl From<CompileError> for ApiError {
    fn from(error: CompileError) -> Self {
        Self(DeviceError::Compile(error))
    }
}

impl From<GraphicsError> for ApiError {
    fn from(error: GraphicsError) -> Self {
        Self(DeviceError::Graphics(error))
    }
}

impl From<RenderError> for ApiError {
    fn from(error: RenderError) -> Self {
        Self(DeviceError::Render(error))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self.0 {
            DeviceError::Validation(definition::ValidationError::UnknownSchema(_))
            | DeviceError::Registry(definition::RegistryError::UnknownDefinition(_)) => {
                StatusCode::NOT_FOUND
            }
            DeviceError::Registry(definition::RegistryError::Delta(
                definition::DeltaError::Conflict { .. },
            )) => StatusCode::CONFLICT,
            DeviceError::Compile(_) => StatusCode::UNPROCESSABLE_ENTITY,
            DeviceError::Render(_) => StatusCode::UNPROCESSABLE_ENTITY,
            DeviceError::CrossCheck { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::BAD_REQUEST,
        };
        (status, self.0.to_string()).into_response()
    }
}
