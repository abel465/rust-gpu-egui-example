#[cfg(feature = "wgpu")]
mod custom3d_wgpu;

#[cfg(feature = "http")]
mod http_app;

#[cfg(feature = "wgpu")]
pub use custom3d_wgpu::Custom3d;

#[cfg(feature = "http")]
pub use http_app::HttpApp;
