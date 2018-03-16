## High Priority

[ ] A `once` keyword that would only output values on `rev` 0
[ ] An `inverse` keyword that would flip all values in a list (xor? or?)
[ ] A word to flip between 0s and ~s used in lists
[ ] Add track type & channel as static arguments to track directve
[ ] Add bjorklund keyword
[ ] Documentation for humans

  [ ] Describe overal architecture and how a program fits in and its life cycle
  [ ] Write some documentation for the keywords

## TODO

[ ] Send MIDI Ctrl events at a variable rate, depending on the number of
    channels, aim for a constant rate of 50-100 a second?
[ ] Functions as values so that they dont have to represented as symbols
    in the language, but can be 'variables' prefixed with `$`
[ ] The distinction between `null` and `0` in generating rhythms with
    `rests`
[ ] Need a glossery of terms (TOS, Cycle, Stack, Heap)
[ ] does `repeat` create a list of `N` values or pushes `N` values onto the
    stack, is there another command that creates a list from N values popped
    off the stack?

## Longterm

[ ] Add a full C FFI api for setting up and running the vm in realtime mode for
    use with python or WASM. An embedable API as it were
[ ] A debugger of sorts
[ ] Some terminal visualisation of the tracks current events (ncurses ascii art
    or something)
[ ] Is this thing strongly typed weakly typed, badly typed?

## Done

[x] Fix tracks going out of time with each other over long periods
[x] Drifting clock, accumulated error (schedule events at real time)
[x] Improve timing, efficiency and accuracy. Juce uses an interesting
    approach apparently used in Tracktion
    Note the losing of thread priority and sleeping

        https://github.com/WeAreROLI/JUCE/blob/master/modules/juce_core/time/juce_Time.cpp#L272-L295

[x] Rename JezErr to Error
[x] Error handling for humans

  [x] Display a real stack trace from when an error occurs at runtime.
  [x] Minimum must include which function
  [x] One day include line and column numbers for parsed tokens? How far away
      is this?

[x] The simul keyword and the concept of subdivision needs fully fleshing out
    cos its not obvious enough. Maybe the output of a track function is a
    (`duration` & `tree`) pair, where tree is an
[x] Value equality for things like sets is gonna be confusing with pairs and
    lists cos a pair could be created but not point to the heap etc.
[x] The distinction between a 'pair' and a 'list' is not clear,
    makes describing 'print_heap' confusing.
[x] Words that operate on and return Pairs but not subdivided cos subdiv rules
    changed
[x] WebSockets Sink for testing in browser
[x] Remove the threads from the sink implementations and move to `main`.
    Hopefully so that sinks can be composed.
[x] Lists OR Sequence?
[x] Add track directive and remove `track` keyword. Should solve declaring
    filters on tracks that 'dont exist' yet and make explaining things easier
[x] A way of passing options & arguments to sinks. (Portmidi specifically)
[x] Integration tests structured by running actual programs and returning
    values from `.def main 0:`. Minimum would check the sieve functions
[x] Need a `modulo` keywrod
[x] `rotate` should allow for a continually growing number to be used
    as the amount (so it should just wrap around)
[x] Version number needs to be brough down to `.version 0` before
    release

https://graphics.stanford.edu/~seander/bithacks.html#SwappingValuesXOR

Swapping individual bits with XOR
unsigned int i, j; // positions of bit sequences to swap
unsigned int n;    // number of consecutive bits in each sequence
unsigned int b;    // bits to swap reside in b
unsigned int r;    // bit-swapped result goes here

unsigned int x = ((b >> i) ^ (b >> j)) & ((1U << n) - 1); // XOR temporary
r = b ^ ((x << i) | (x << j));


import numbers

def from_string(s):
    return BitArray([int(t.strip()) for t in list(s) if t.strip()])

def from_num(s):
    return from_string(bin(s)[2:])

def b(s):
    if isinstance(s, numbers.Number):
        return from_num(s)
    return from_string(s)


initial   [1 1 1 1 1 0 0 0 0 0 0 0 0]                   0
          [1 0] [1 0] [1 0] [1 0] [1 0] [0] [0] [0]     1
          [1 0 0] [1 0 0] [1 0 0] [1 0] [1 0]           2
          [1 0 0 1 0] [1 0 0 1 0] [1 0 0]               3

initial   1 1 1 1 1 0 0 0 0 0 0 0 0         5 8         8 - 5 = 3
          1 0 1 0 1 0 1 0 1 0 0 0 0         5 3         5 - 3 = 2
          1 0 0 1 0 0 1 0 0 1 0 1 0         3 2         3 - 2 = 1
          1 0 0 1 0 1 0 0 1 0 1 0 0         2 1         2 - 1 = 1

       OR'ing will always ensure 1bit is at the front
       1  0  0  0  0  0  0  0  0  0  0  0  0 = 4096

      00 01 02 03 04 05 06 07 08 09 10 11 12
       1  1  1  1  1  0  0  0  0  0  0  0  0 = 7936

XOR    1  0  1  0  1  0  1  0  1  0  1  0  1 = 5461
=      0  1  0  1  0  0  1  0  1  0  1  0  1 = 2645

XOR    0  1  0  1  0  1  0  1  0  1  0  1  0 = 2730
=      1  0  1  0  1  1  0  1  0  1  0  1  0 = 5546

============================================
       1  0  1  0  1  0  1  0  1  0  0  0  0



1 1 1 1 1 0 0 0 0 0 0 0 0         5 8         8 - 5 = 3
  x             x
      y     y
1 0 1 0 1 0 1 0 1 0 0 0 0         5 8         8 - 5 = 3


1 0 1 0 1 0 1 0 1 0 0 0 0         5 3         5 - 3 = 2
    x                 x
      y             y
1 0 0 1 0 0 1 0 0 1 0 1 0
