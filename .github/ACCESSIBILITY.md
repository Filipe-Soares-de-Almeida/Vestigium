# Accessibility

Vestigium is for anyone whose PC freezes, so the window has to work for the people using it. This page says
what is built in today, what is known to be missing and how to report a barrier.

The window has not had a formal accessibility audit, and it has not been tested with a screen reader yet.
Everything under "What is built in" comes from how the window is written, not from testing with assistive
technology.

## What is built in

- The window follows the Windows light or dark setting until you choose one, and offers four color themes.
- Animations and transitions stop when Windows asks for reduced motion.
- Keyboard focus is shown with a visible outline.
- The charts can be read with the keyboard: the arrow keys move along the chart, plus and minus zoom, and
  each position has a text readout.
- The window uses ARIA labels, roles and states on its controls, and the page language follows the language
  you choose, English or Brazilian Portuguese.

## Supported environments

Windows 10 22H2 and Windows 11 on x64. The window is rendered by the Microsoft WebView2 runtime and works
with a keyboard and a mouse.

## Known barriers

- The window has not been tested with screen readers such as Narrator or NVDA, so how well the labels and the
  chart readouts are announced is not known.
- Windows contrast themes have not been tested. Only one marker has a rule for them, and the charts are drawn
  on a canvas, which keeps its own colors.
- The window has not had a formal accessibility audit.

## Contributing

A change to the window keeps the keyboard working, keeps text readable against its background and does not
rely on color alone to tell states apart.

## Reporting a barrier

Open an
[Accessibility barrier](https://github.com/Filipe-Soares-de-Almeida/Vestigium/issues/new?template=05-accessibility.yml)
issue. You do not need to know the cause or the fix.

## Ownership

The maintainer owns accessibility.
