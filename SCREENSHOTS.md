To update `screenshot-git-merge.png`, you should be running
[iTerm2](https://iterm2.com/) and have [`moar`](https://github.com/walles/moar)
as your pager.

Then:

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
