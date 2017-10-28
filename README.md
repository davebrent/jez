# Jez

[![Build Status](https://travis-ci.org/davebrent/jez.svg?branch=master)](https://travis-ci.org/davebrent/jez)

A stack machine for sequencing things

    $ cat <<EOF | jez
      .verson 1

      .def main 0:
        ['track1] tracks

      .def track1 0:
        [1 ~ 2 ~] 1000 0 127 midi_out
    EOF
