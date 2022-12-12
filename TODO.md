- Add clock device
- Add bus device
- Allow for links to be created with anchor points
- Make a tutorial
- Move UI (TopPanel & BottomPanel) code from log_sim_gui to the integrations
- Remove CreateApp, create Keybinds struct
- Switch scene graphics to WGPU Buffers & shaders
- Rename to lahsim (Logic and Arithmetic Hardware Simulator)

have a library `lahsim_core`, which handles updating and drawing the Scene,
and a binary `lahsim`, which draws a UI around & above the Scene
