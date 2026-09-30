//! A hot-reloadable headless graphics device, served over http rpc.
//!
//! # What this crate is
//!
//! The device owns a [`Registry`](definition::Registry) of schemas and definitions, a
//! [`Compiler`] that turns a scene into graphics instructions, and a [`Renderer`] that
//! turns those instructions into pixels. Agents read and write documents; everything else
//! in the workspace is a function of them.
//!
//! # Cross-checking
//!
//! A compiler emits its own delta rather than having one diffed centrally, so nothing else
//! guarantees the delta reproduces the output it came with. The device therefore performs
//! a full recompile and compares whenever it is built in debug or test mode, and a
//! mismatch is reported as an error instead of silently corrupting the stored graphics.
//! In release the emitted delta is trusted, because re-compiling on every reload would
//! defeat the point of compiling incrementally.
//!
//! # Rendering
//!
//! [`NoopRenderer`] is always available and always reports itself as unavailable, so the
//! server and the reload path stay exercisable with no gpu. With the `wgpu` feature
//! enabled, [`WgpuRenderer`] is selected instead when an adapter is present. Every render
//! response names the renderer that produced it, so a placeholder is never mistaken for
//! real pixels.
//!
//! # Example
//!
//! ```
//! use definition::{Definition, DefinitionIdentifier, SchemaIdentifier, Value};
//! use whiting_device::{Device, NoopRenderer};
//! use scene::{register_scene_schema, Compiler, ReferenceScene, REFERENCE_SCENE_SCHEMA};
//!
//! let graphics_identifier = DefinitionIdentifier::new("scene.graphics");
//! let instance_identifier = DefinitionIdentifier::new("scene.instance");
//! let mut device = Device::new(
//!     Box::new(ReferenceScene::new(graphics_identifier.clone(), instance_identifier.clone())),
//!     Box::new(NoopRenderer),
//! );
//! device.install_graphics_schemas();
//! register_scene_schema(device.registry_mut());
//!
//! let source = DefinitionIdentifier::new("scene.source");
//! device.registry_mut().insert_definition(Definition::new(
//!     source.clone(),
//!     SchemaIdentifier::new(REFERENCE_SCENE_SCHEMA),
//!     Value::structure([
//!         ("name".to_owned(), Value::String("backdrop".to_owned())),
//!         ("layers".to_owned(), Value::List(vec![Value::variant("Layer", [
//!             ("label".to_owned(), Value::String("sky".to_owned())),
//!             ("shader".to_owned(), Value::String("sky".to_owned())),
//!             ("clear_color".to_owned(), Value::List(
//!                 [0.1, 0.2, 0.3, 1.0].into_iter().map(Value::Float).collect(),
//!             )),
//!         ])])),
//!     ]),
//! )).expect("the definition is new");
//!
//! // A first reload creates both outputs and the emitted deltas are empty, because there
//! // is no previous state to delta from.
//! let first = device.reload(&source).expect("the scene compiles");
//! assert!(first.graphics_created);
//! assert!(first.cross_checked);
//! assert_eq!(first.graphics_operations, 0);
//!
//! // A second reload with nothing changed emits an empty delta and advances the version
//! // by the one recorded apply.
//! let second = device.reload(&source).expect("the scene recompiles");
//! assert!(!second.graphics_created);
//! assert_eq!(second.graphics_operations, 0);
//!
//! // The no-operation renderer always says it is unavailable, so a placeholder frame can
//! // never be mistaken for real pixels.
//! assert_eq!(device.renderer().name(), "noop");
//! assert!(!device.renderer().available());
//! let frame = device.render(&graphics_identifier).expect("the frame renders");
//! assert_eq!(frame.renderer, "noop");
//! // The size comes from the target the first pass binds, resolved through the instance.
//! assert_eq!((frame.frame.width, frame.frame.height), (512, 512));
//!
//! // The compiler identifies the instance the renderer binds against.
//! assert_eq!(device.compiler().instance_identifier(), &instance_identifier);
//! ```

#[cfg(feature = "png")]
mod image;
mod render;
mod rpc;
mod snapshot;
mod state;

#[cfg(feature = "wgpu")]
mod wgpu_renderer;

pub use render::{Frame, NoopRenderer, RenderError, RenderedFrame, Renderer};
pub use rpc::{router, serve};
pub use snapshot::{NamedDefinition, NamedSchema, Snapshot};
pub use state::{Device, DeviceError, ReloadReport};

#[cfg(feature = "wgpu")]
pub use wgpu_renderer::WgpuRenderer;
