# Jez

[![Build Status][master]](https://travis-ci.org/davebrent/jez)

  [master]: https://travis-ci.org/davebrent/jez.svg?branch=master

Jez is a text-based step sequencer for generative music.

Jez implements a domain specific language for expressing step sequences and
provides a command-line tool for their playback over Open Sound Control and
MIDI.

Nested step sequences are supported, for creating more rhythmically complex
patterns, and a range of built-in keywords and track effects are provided for
adding further variation.

**Features**

* Deterministic.
* Support for Open Sound Control and MIDI.
* Range of built-in keywords & track effects for manipulating sequences.
* Support for user defined keywords.
* Light on system dependencies.

**Plans**

* C API for FFI usage (primarily for Web Assembly).
* MIDI clock support & other forms of synchronisation.
* Better documentation and examples.
