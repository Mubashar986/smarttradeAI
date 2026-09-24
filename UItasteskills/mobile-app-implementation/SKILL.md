---
name: mobile-app-implementation
description: Translates an approved mobile screen flow and semantic token system into a platform-native implementation plan for SwiftUI, Jetpack Compose, React Native, or Flutter.
---

# Mobile App Implementation Bridge

Use this skill after `imagegen-frontend-mobile` has produced an approved screen
flow. It turns a visual concept into an implementation plan; it does not
replace the platform's official design guidance or invent an application stack.

## Required input

- A declared target: `swiftui`, `jetpack-compose`, `react-native`, or `flutter`.
- An approved mobile flow with the primary path, error path, empty state, and
  loading state.
- A semantic token contract from Picasso.
- Real data contracts or an explicit backend gap record from Escher.

If the target platform is not declared, stop at the implementation plan. Do
not produce generic cross-platform code and call it native.

## Platform translation

### SwiftUI

- Follow Apple Human Interface Guidelines for navigation, sheets, typography,
  dynamic type, safe areas, and accessibility labels.
- Prefer native navigation stacks, tab views, lists, and sheets before custom
  containers.
- Test larger accessibility text sizes and reduced motion.

### Jetpack Compose

- Follow Material 3 guidance for navigation, insets, typography, color roles,
  touch feedback, and semantic descriptions.
- Use Compose state hoisting and scaffold patterns rather than web-style view
  hierarchies.
- Verify system bars, gesture navigation, font scaling, and dark theme.

### React Native

- Use platform-aware navigation and safe-area primitives. Do not force a web
  layout model into native screens.
- Keep animation work off the JS render path where the selected library allows.
- Verify on iOS and Android when the product claims both platforms.

### Flutter

- Choose Material 3, Cupertino, or a documented adaptive component strategy.
- Respect `SafeArea`, text scaling, semantic labels, focus order, and system
  navigation regions.
- Keep screen state, routing, and design tokens separate from widget styling.

## Mobile implementation rules

- Build the primary flow before decorative motion.
- Preserve the approved navigation model. Tabs are for peer destinations,
  stacks are for drill-down, and sheets are for focused secondary actions.
- Support touch targets of at least 44 by 44 points or density-independent
  pixels, unless the platform standard requires more.
- Respect safe-area, status-bar, home-indicator, keyboard, and gesture regions.
- Support dynamic type or platform font scaling. Do not clip text to preserve a
  visual mockup.
- Provide loading, empty, error, offline, and permission-denied states where
  the data or capability requires them.
- Honor reduced motion. Animate transforms and opacity; do not use motion that
  blocks task completion or makes a gesture ambiguous.
- Use the semantic token system. Convert tokens to native color, type,
  spacing, radius, and elevation representations instead of copying CSS values.

## Required artifact

Create `mobile-implementation-plan.md` containing:

1. Target platform and implementation framework.
2. Screen-to-route map and navigation ownership.
3. Token translation table.
4. Screen state matrix: loading, empty, error, offline, permission denied.
5. Data contract dependencies and known backend gaps.
6. Safe-area, touch-target, dynamic-type, dark-mode, and reduced-motion rules.
7. Verification plan using platform previews or simulator screenshots.

## Completion gate

Do not mark a mobile UI task complete until screenshots or recordings cover the
primary flow on the target platform, a small device, a large device, increased
text size, and reduced-motion behavior when applicable.
