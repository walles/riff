# Screenshot updating instructions

Screenshots are done using:

- Terminal: [iTerm2](https://iterm2.com/)
- Pager: [`moor`](https://github.com/walles/moor)

## `screenshot.png`

Scale your window to 92x28, then:

- Get the moor source code: <https://github.com/walles/moor>
- Do: `git -C ../moor show 9c91399309 | cargo run`

## `screenshot-git-merge.png`

1. Scale your terminal window to 65x17
2. Copy the below example to the clipboard
3. `pbpaste | cargo run`
4. Screenshot the window and store it as `screenshot-git-merge.png`

```diff
diff --cc hello.rb
index 0399cd5,59727f0..0000000
--- a/hello.rb
+++ b/hello.rb
@@@ -1,7 -1,7 +1,7 @@@
  #! /usr/bin/env ruby

  def hello
-   puts 'hola world'
 -  puts 'hello mundo'
++  puts 'hola mundo'
  end

  hello()
```

## `screenshot-diff2-conflict.png`

1. Scale your window to 72x10
2. `cargo run < testdata/conflict-markers.txt`
3. Screenshot the window and store it as `screenshot-diff2-conflict.png`
