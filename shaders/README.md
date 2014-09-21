The shaders are run through an extra preprocessor before being passed to OpenGL.
This lets us splice in data based on variable names between `$` tokens.
The values for these variables are passed to the shader loading functions.
