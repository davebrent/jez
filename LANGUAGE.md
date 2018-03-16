# Language documentation

Jez programs define a set of named `tracks` that output notes or cc messages,
expressed as step sequences.

## Version

All programs must specify the language version it expects Jez to support. This
should be the first directive of every program.

## Tracks

Tracks are defined using the `.track` directive taking the form of

    .track <NAME> :
      <CODE> ...

Code is written using reverse polish notation, operating on a stack.

## Functions

Functions may be written using the `.def` directive taking the form of

    .def <NAME> <NUMBER_OF_ARGUMENTS> :
      <CODE>...

A functions return value is defined as the last value on the top of its stack.
The function may then be `called` in the same way as other keywords. The
functions stack will be initialised with the _number of arguments_ popped off
its callee's stack.

## Tutorial

Hello world

``` .track t1: [ 2000 (96 64 64 64) ] ```

Jez supports simple data types such as numbers, strings & lists, lists enclose


Sequences are defined by enclosing values within `(` `)` within a track,
delimited with `(` `)`, the contents of

## Concepts

The building block of the sequencer is the track, tracks in Jez run
independently from one another (there is no global tempo control). Tracks are
made up of a series

Sequences are represented as a series of `instructions` for a small virtual
machine. Instructions are grouped into tracks, that are evaluated periodically
to produce `events`. This allows for complex patterns to emerge very quickly
from very simple series of instructions.

The period during with a track is evaluated is called the `duration`, and is
defined in milliseconds. The duration controls the _speed_ of a track and is
similar to BPM & tempo controls found on other pieces of software. Track
durations may be independent from each other to allow for polymetric
sequencing.

Events may consist of note or controller messages and can be routed via MIDI or
Open Sound Control to control other programs or external hardware.

Track instructions are intrepreted by a "stack machine" to produce a "tree of
events". The tree structure is "subdivided" against a "duration" to produce a
series of events over time. For more interesting effects there are "keywords"
that manipulate the stack.

The dynamic nature of keywords allow for varying events to be produced from the
same set of instructions thus creating more interesting sequences.

Tracks may have "effects" (called `filters`) that operate on the events
outputted by a track. Such as quantising pitches to defined scales etc.

### Glossery

* *Track* A function that may be called periodlicly to generate a list of
  events over a specified duration.

* *Cycle* The count of home many times a track function has been called.

* *Heap* Contigous list of values that may grow during the execution of a
  track function but is then cleared when the track function call ends.

* *Stack* The functions call stack

* *TOS* Top most value on the stack

## Architecture

Jez's architecture is based around the idea of a 'text-based' sequencer, like
most sequencers it has a concept of 'tracks', and 'events'. A track is a
function, that when called, returns a list of 'events' and a time duration.
Events may be 'note' or 'controller' events and no events will fall outside of
this time duration.

The list of events is constructed by recursively subdividing a tree structure
of 'expressions'.

A tracks time duration may vary for more interesting patterns. When a tracks
duration is finished, its expressions will be re-evaluated and the process
repeats.

## Reference

### Contents

* [Directives](#directives) [Instuctions](#Instructions) [Symbols](#symbols)
* [Math](#math) [Stack](#stack) [Debug](#debug) [Cycle](#cycle) [List](#list)
* [Set](#set) [Probability](#probability) [Rhythms](#rhythms) [Curves](#curves)
* [Audio](#audio) [MIDI](#midi) [Filters](#filters)

### Directives

#### .version

Specify the minimum expected language version. Required as a programs first
directive.

    .version <NUMBER>

#### .globals

Declare and initiaze multiple global variables.

    .globals <<KEY> = <VALUE>>...

#### .def

Define a new keyword.

    .def <NAME> <NUM_ARGS> : <INSTRUCTION>...

#### .track

Define a new track/entrypoint to the program

    .track <NAME> : <INSTRUCTION>...

## Instructions

### Symbols

#### [ ]

Create a list.

    [ <VALUE> ... ]

#### ~

Push a null/rest value to the stack

    ~ ### Stack

#### pair

Create a pair of two values.

    <A> <B> pair

#### drop

Pop the 'TOS'

    <VALUE> drop

#### dup

Duplicate the 'TOS'

    <VALUE> dup

#### swap

Swap the top two values.

    <A> <B> swap

### Math

#### add

Add two numbers.

    <LHS> <RHS> add

#### divide

Divide two numbers.

    <LHS> <RHS> divide

#### multiply

Multiply two numbers.

    <LHS> <RHS> divide

#### subtract

Subtract two numbers.

    <LHS> <RHS> subtract

### Debug

#### print

Print the 'TOS', leaving the value in place.

    <VALUE> print

#### print_heap

Print the heap values pointed to by a 'pair'.

    <LIST> print_heap

### Cycle

#### simul

Signal to the subdivision algorithm not to subdivide the list but instead send
all elements at the same time.

    <LIST> simul

#### tracks

Define a list of track functions. Required to perform realtime output.

    <LIST> tracks

#### revision

Push the current tracks cycle revision onto the stack.

    revision

### List

#### repeat

Repeat a value *N* times

    <VALUE> <N> repeat

#### every

Push a value on the stack every *N* cycles

    <VALUE> <N> every

#### reverse

Reverse a list pushing the result back onto the stack

    <LIST> reverse

#### rotate

Rotate items in a list (or shift?) by a given amount

    <LIST> <AMOUNT> rotate

#### cycle

Cycles through elements in a list every cycle

    <LIST> cycle

#### palindrome

Reverses a list every *other* cycle.

    <LIST> palindrome

#### range

Create a range of integers.

    <MIN> <MAX> range

#### gray_code

Return the corresponding Gray Code for an integer.

    <NUMBER> gray_code

#### bin_list

Return the binary representation of a number as a binary list.

    <NUMBER> bin_list

### Set

#### intersection

Return the intersection of two lists.

    <A> <B> intersection

#### union

Return the union of two lists.

    <A> <B> union

#### symmetric_difference

Return the symmetric difference (XOR) of two lists.

    <A> <B> symmetric_difference

### Probability

#### rand_seed

Seed the random number generator used by all other functions. *Seed* must be a
number.

    <SEED> rand_seed

#### rand_range

Generate a random integer within a given range

    <MIN> <MAX> rand_range

#### shuffle

Shuffles items in a list pushing the result back onto the stack

    <LIST> shuffle

#### degrade

Randomly set items in a list to `~` (or null?)

    <LIST> degrade

### Rhythms

#### hop_jump

Create a rhythm that satisfies the rhythmic oddity property.

    <ONSETS> <PULSES> <HOPSIZE> hop_jump

#### inter_onset

Return a new list containing the difference between consecutive elements

    <LIST> inter_onset

#### onsets

Return a binary rhythm from an onset list.

    <LIST> onsets

#### sieve

Apply a Zenakis residual class to an integer sequence. Returns a new integer
sequence.

    <SEQUENCE> <RESIDUAL_CLASS> sieve

### Curves

#### linear

Create a linear ramp between two numbers.

    [ <FROM> <TO> ] linear

### Midi

#### midi_out

### Filters

#### markov_filter #### pitch_quantize_filter

## Prior Art

For more mature projects whos presense very much influenced Jez please see
[SuperCollider][6] & [Tidal][7] for this is very much still alpha quality
software.

  [6]: https://supercollider.github.io/
  [7]: https://tidalcycles.org
