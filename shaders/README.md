The shaders are run through an extra preprocessor before being passed to OpenGL.

Right now, it lets us compile the shaders with different functionalities enabled
based on flags passed to the shader loading functions in `src/shader.rs`.
The syntax is `$if`, an optional `$else`, and `$end`, insensitive to indenting.

**Right not, `$if`s can NOT be nested**

The preprocessor is written to accommodate correct shaders correctly,
and makes some effort to notice errors, but it may produce strange behavior
if given strange inputs, e.g. `$else` without an `$if`.
