## Release 221113:
- Added this change-log
- Added name editor for scene outputs
- Added support for light mode in scene input/output name editor
- Fixed placing devices being offset
- Fixed placing inputs/outputs being offset
- Fixed device name size not scaling
- Improved errors for reloading missing settings
- Added buttons for deleting and modifying presets in the "Presets" menu
- Can now delete hovered links (Backspace)
- Can now stack inputs and outputs (ArrowDown)

## Release 221116
- Fixed device inputs/outputs not scaling
- Added background to scene outputs in a stack
- Added high contrast support for light mode
- Fixed input and output groups not saving correctly
- Fixed dragging devices not scaling
- Added optimization for creating combinational devices
- Improved link interaction box
- Added setting: "link width"
- Fixed device links interfering with device pins
- Hid the debug settings and debug menu behind a hidden setting

## Release 221125
- Fixed scene dragging when menu is open
- Can now start creating multiple links at once
- Can now delete all links on device output by pressing Backspace on the pin
- Can now place held presets when pointer is over an item on scene
- Added "auto link" : automatically start/finish a link when you hover a pin
- Can now un-stack input and output groups by pressing Up
- Fixed scene inputs and outputs not scaling
- Fixed crash when deleting devices that are linked to from devices
- Can no longer connect two links to the same target
- Fixed zooming not centering at your pointer after you've moved the scene
- Added minimum and maximum zoom
- Removed the preset picker
- Added new preset placer (press Space to activate)
- Placing presets via the context menu now places them where you opened the context menu

- Added hover texts to some UI items
- Changed window title and save directory to LogSimGUI
- Added button to open the config folder in a file viewer
- Added button to save the current state of the app (the presets, settings, and scene)
- Improved Presets menu

- Improved error messages for loading invalid settings, presets, or scene
- Improved file save times
- Saving now also saves the state of the scene
- All presets are stored in individual files now (to reduce pauses from auto-save)
- The settings are now saved with RON instead of JSON

## Release 221203
- Can now select multiple chips by shift clicking on them
- Can now duplicate selected chips with `⌘/Ctrl + D`
- Preset placer is now always shown and some visual changes to the preset placer
- Preset placer position is now saved in settings
- Preset placer now shows recently placed presets if the field is empty
- Can now delete/stack inputs/outputs in their context menu
- Increased maximum zoom
- Tweaked some visuals
- Creating a preset from a scene with loops will now visually show an error message
- Linking a scene input to a scene output will now visually show an error message
- Can now move the scene by scrolling

- Clear button now shows how many items are in scene (if any)
- Replaced speed slider with a + and - button
- Can now place presets from the presets menu

- Saving/loading presets no longer uses the `__index.ron` file
- Improved error messages for loading invalid settings, presets, or scene

## Latest (On GitHub)
- Improved simulation performance
- Added "version" setting
- Removed "dev_options" setting
