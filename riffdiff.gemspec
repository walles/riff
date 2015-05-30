$LOAD_PATH.unshift File.join(File.absolute_path(__dir__), 'lib')
require 'version'

Gem::Specification.new do |s|
  extend Version

  s.name        = 'riffdiff'
  s.version     = semantify_git_version(git_version)
  s.summary     = 'A diff highlighter showing what parts of lines have changed'
  s.description = %{== Riff
Riff is a wrapper around diff that highlights not only which lines have changed,
but also which parts of the lines that have changed.

= Usage

 git diff | riff

Or if you do...

 git config --global pager.diff riff
 git config --global pager.show riff

... then all future <code>git diff</code>s and <code>git show</code>s will be
refined.
}
  s.authors     = ['Johan Walles']
  s.email       = 'johan.walles@gmail.com'
  s.homepage    = 'http://github.com/walles/riff'
  s.license     = 'MIT'

  s.files         = `git ls-files`.split("\n")
  s.test_files    = `git ls-files -- {test,spec,features}/*`.split("\n")
  s.executables   = ['riff']
  s.require_paths = ['lib']
end
