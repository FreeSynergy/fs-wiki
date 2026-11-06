# Touch Gestures

Touch gestures are interaction methods for touchscreen devices (and related
pointers: mouse, pen, motion sensors). Gestures are **inputs** – the visual
answer to them is an [animation](../animations/).

Gestures are grouped into six families (mirroring the animation categories).

## Families

| Family | Principle | Gestures |
|--------|-----------|----------|
| [Tap (tap)](tap/) | Short touch without a path, mostly replaces clicks – from one finger, multipoint-capable | 6 |
| [Swipe (swipe)](swipe/) | Guided movement in one direction – from one finger, multipoint-capable | 6 |
| [Transform (transform)](transform/) | Two+ pointers relative to each other (never one), the count is a setting, transforms content | 3 |
| [Special (special)](special/) | No classic touched element: hover, biometric sensor | 2 |
| [Device (device)](device/) | Movement of the device itself (sensors, no pointers) | 4 |
| [Combined (combined)](combined/) | Composite gestures with their own logic | 2 |

## Pointer Count: Single-Point and Multipoint

A touch sensor reports every simultaneous contact; depending on the device,
five to ten fingers are detected at once. From this follow the rules for all
gestures:

- **The base gestures are multipoint-capable.** Tap, double tap, long press,
  force press, swipe, flick, drag, edge swipe and the combined gestures work
  with any number of fingers up to the device limit – recognised on the
  primary pointer or on the centre of all pointers. “One finger” is the
  minimum, not the maximum.
- **Multipoint always covers every finger count.** A gesture carries no limit
  of its own; the ceiling is always the device limit.
- **Transform and multi-finger gestures cannot run single-point.** Pinch,
  spread and rotate require two pointers by necessity (distance and angle
  only exist from two on), and the multi-finger variants of tap, long press
  and swipe do by definition.
- **Three exceptions without multipoint:** `draw-gesture` (one path is one
  connected stroke – further fingers create their own, separate paths),
  `hover` (no contact; a hovering multi-finger pointer does not exist) and
  `fingerprint` (the sensor reads exactly one finger pad).
- **Device gestures know no pointer count:** shake, tilt, flip and wave run
  on sensors, touches play no role.

| Family / Gesture | Single-Point | Multipoint | Basis |
|------------------|--------------|------------|-------|
| tap: tap, double-tap, long-press, force-press | ✓ | ✓ up to device limit | primary pointer / pointer centre |
| tap: multi-finger variants | ✗ | ✓ from two | at least two fingers needed |
| swipe: swipe, flick, drag, edge-swipe | ✓ | ✓ up to device limit | primary pointer / pointer centre |
| swipe: draw-gesture | ✓ | ✗ | one path = one stroke |
| swipe: multi-finger-swipe | ✗ | ✓ from two | at least two fingers needed |
| transform (all) | ✗ | ✓ up to device limit | at least two pointers needed |
| special: hover | ✓ | ✗ | only one hovering pointer |
| special: fingerprint | ✓ (exactly one) | ✗ | sensor reads one finger pad |
| device (all) | not applicable | not applicable | sensors instead of pointers |
| combined | ✓ | ✓ | follows its base gestures (drag) |

## All Gestures

| Gesture | Family | Description | Usage |
|---------|--------|-------------|-------|
| **Tap** | tap | Brief touch | Button click, item selection |
| **Double Tap** | tap | Two quick touches | Zoom, open |
| **Long Press** | tap | Long touch | Context menu, marking |
| **Force Press** | tap | Pressure via pressure sensor | Peek & pop, context |
| **Multi-Finger Tap** | tap | Tapping with several fingers | Secondary action |
| **Multi-Finger Long Press** | tap | Long press with several fingers | Secondary menu, multiselect |
| **Swipe** | swipe | Swiping in one direction | Navigation, switching |
| **Flick** | swipe | Fast swipe with lift-off momentum | Throw-scrolling |
| **Drag** | swipe | Dragging while pressed | Reorder, scroll |
| **Edge Swipe** | swipe | Swipe from the screen edge | Back, system bars |
| **Multi-Finger Swipe** | swipe | Swiping with several fingers | App/workspace switching |
| **Draw Gesture** | swipe | Drawing a shape/letter | Shortcuts, commands |
| **Pinch** | transform | Pulling pointers together (from two) | Zoom out |
| **Spread** | transform | Moving pointers apart (from two) | Zoom in |
| **Rotate** | transform | Turning pointers (from two) | Rotation |
| **Hover** | special | Pointer over element without contact | Tooltip, preview |
| **Fingerprint** | special | Touching a biometric sensor | Unlocking, authorising |
| **Shake** | device | Quick back-and-forth movement | Undo, error feedback, reshuffle |
| **Tilt** | device | Tilting the device | Steering, parallax |
| **Flip** | device | Turning the device over | Mute, do not disturb |
| **Wave** | device | Waving a hand over a sensor | Answer calls, air gestures |
| **Pull to Refresh** | combined | Pulling at the list top edge | Refresh |
| **Drag and Drop** | combined | Grab, move, drop | Move, combine |

## Gesture Attributes

| Attribute | Type | Description |
|-----------|------|-------------|
| `onTap` | function | Executed on Tap |
| `onDoubleTap` | function | Executed on Double Tap |
| `onLongPress` | function | Executed on Long Press |
| `onSwipe` | function | Executed on Swipe |
| `onFlick` | function | Executed on Flick |
| `onDragStart` | function | Executed on drag start |
| `onDragEnd` | function | Executed on drag end |
| `onEdgeSwipe` | function | Executed on Edge Swipe |
| `onForcePress` | function | Executed on Force Press |
| `onHoverStart` / `onHoverEnd` | function | Hover starts/ends |
| `onFingerprint` | function | Executed after a successful fingerprint scan |
| `onFingerprintFail` | function | Executed when the scan finds no match |
| `onDrawGesture` | function | Executed after a recognized shape |
| `onPinch` | function | Executed on Pinch |
| `onSpread` | function | Executed on Spread |
| `onRotate` | function | Executed on Rotate |
| `onMultiFingerTap` | function | Executed on Multi-Finger Tap |
| `onMultiFingerLongPress` | function | Executed on Multi-Finger Long Press |
| `onMultiFingerSwipe` | function | Executed on Multi-Finger Swipe |
| `onShake` | function | Executed on Shake |
| `onTilt` | function | Executed on tilt |
| `onFlip` | function | Executed on Flip |
| `onWave` | function | Executed on Wave |
| `onRefresh` | function | Executed on Pull to Refresh |
| `onDrop` | function | Executed on drop (Drag and Drop) |

## Configuration

| Attribute | Type | Description |
|-----------|------|-------------|
| `tapThreshold` | number | Time limit for Tap in ms |
| `longPressThreshold` | number | Time limit for Long Press in ms |
| `swipeThreshold` | number | Minimum distance for Swipe |
| `velocityThreshold` | number | Minimum velocity for Flick in px/s |
| `edgeSize` | number | Width of the edge zone for Edge Swipe in px |
| `pointerCount` | number | Minimum pointer count for the gesture (base gestures: 1, transform and multi-finger gestures: 2 or 3 …) – limited above by the device |
| `shakeThreshold` | number | Minimum acceleration/movement for Shake |
| `minShakes` | number | Minimum number of direction changes for Shake |
| `deadZone` | number | Degree tolerance for Tilt |
| `matchThreshold` | number | Shape similarity for Draw Gesture (0–1) |
| `enabled` | boolean | Enable gestures? |

## Standards and Accessibility

| Rule | Meaning |
|------|---------|
| WCAG 2.5.1 Pointer Gestures | Multi-point and path gestures need a single-pointer alternative |
| WCAG 2.5.2 Pointer Cancellation | Execute the action on release, not on press |
| WCAG 2.5.4 Motion Actuation | Sensor gestures (shake, tilt, flip, wave) need a UI alternative |
| WCAG 2.5.7 Dragging Movements | Drag needs a single-pointer alternative |
| `touch-action` (CSS) | Control browser-native gestures (pan, pinch-zoom) |

Because gestures are never self-explanatory, the same rule applies everywhere:
make the same action reachable via button, menu or keyboard as well.

## See also

- Families: [tap](tap/) · [swipe](swipe/) · [transform](transform/) · [special](special/) · [device](device/) · [combined](combined/)
- [Animations](../animations/) — visual answer to gestures
- [Conventions](../../data/conventions.md) — file and naming rules
