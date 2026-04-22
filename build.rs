use std::env;

fn main() {
  println!("cargo:rerun-if-changed=build.rs");
  println!("cargo:rerun-if-changed=assets/activity.ico");

  let target = env::var("TARGET").unwrap_or_default();
  if !target.contains("windows") {
    return;
  }

  let mut res = winres::WindowsResource::new();
  res.set_icon("assets/activity.ico");
  // Optional metadata for better Windows shell presentation.
  res.set("ProductName", "dictation-rs");
  res.set("FileDescription", "dictation-rs desktop dictation");
  res.set("OriginalFilename", "dictation.exe");
  res.set("InternalName", "dictation");
  res
    .compile()
    .expect("failed to compile Windows resources with winres");
}
