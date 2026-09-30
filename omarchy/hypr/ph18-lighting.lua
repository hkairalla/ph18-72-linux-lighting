-- Window rules for the PH18-72 lighting GUI (Omarchy's Hyprland config, Lua).
-- The GUI's Wayland app id is "ph18-lighting" (set in app/.../main.py), so it can be
-- matched here. Uses Omarchy's o.window() helper (see $OMARCHY_PATH/default/hypr/helpers.lua).
-- Float it, center it and give it a sensible size instead of tiling it into a column.
o.window("ph18-lighting", { float = true })
o.window("ph18-lighting", { center = true })
o.window("ph18-lighting", { size = { 1100, 720 } })
