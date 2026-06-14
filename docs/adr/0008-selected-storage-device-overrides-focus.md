# Selected Storage Device Overrides Focus Alarm

Status: proposed

A Storage Health Display shows one storage device at a time. By default it surfaces the Focus Storage Device (the worst-health device), but once a user explicitly picks a Selected Storage Device the whole overview — including the top-level status icon — follows that selection, even when another device is in a worse state. Overall danger is instead surfaced through per-device health badges in the device selector list, so a more critical device elsewhere stays visible without forcing the view back to it. We chose "selection wins, badges raise the alarm" over keeping the top icon as an always-worst aggregate, because mixing an aggregate icon with a per-device label and metrics panel is confusing, and it keeps the interaction consistent with the GPU selector. Until the user makes an explicit selection, the default still shows the Focus Storage Device, so the header keeps acting as an overall alarm out of the box.

## Considered Options

- **Header follows selection (chosen).** Consistent with the GPU selector; the whole panel describes one device. Overall danger relies on the selector badges.
- **Hybrid escalation.** Normally follow the selection, but escalate the top icon to the overall worst when a non-selected device is critical. Safer, but the icon and the panel can then describe different devices.
- **Keep aggregate alarm.** Top icon always reflects the overall worst; only the label and metrics follow the selection. Preserves the alarm but leaves the icon and label pointing at different devices.

## Consequences

With a device selected, the overview header no longer reflects overall health. A reviewer who sees a healthy ("good") header while another device is critical should not treat it as a bug — the per-device badges in the selector are the overall-health signal. This is a deliberate behavioral asymmetry, not an oversight.
