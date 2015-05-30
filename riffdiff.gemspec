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

<tt>git diff | riff</tt>

Or if you do...

<tt>git config --global pager.diff riff</tt>

<tt>git config --global pager.show riff</tt>

... then all future <tt>git diff</tt>s and <tt>git show</tt>s will be
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
