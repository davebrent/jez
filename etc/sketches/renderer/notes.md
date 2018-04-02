Problem, how to texture a predrawn scene
Is this a problem?

https://mathoverflow.net/questions/69244/encoding-n-natural-numbers-into-one-and-back

R - Shape ID / Material index
G -
B - Color palete index (pixel art)
A - Depth

    >>> import math
    >>>
    >>> def encode(a, b):
    ...     return (math.pow(a + b, 2) + (3 * a) + b) / 2
    ...
    >>> encode(5, 6)
    71.0
    >>> def decode(n):
    ...     s = math.floor( math.sqrt(2 * n) )
    ...     a = ((2 * n) - (math.pow(s, 2)) - s) / 2
    ...     b = s - a
    ...     return (a, b)
    ...
    >>> decode(71)
    (5.0, 6.0)
    >>>
    >>> encode(10, 10)
    220.0
