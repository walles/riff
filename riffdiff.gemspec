$LOAD_PATH.unshift File.join(__dir__, 'lib')
require 'version'

Gem::Specification.new do |s|
  extend Version

  s.name        = 'riffdiff'
  s.version     = semantic_version
  s.summary     = 'A diff highlighter showing what parts of lines have changed'
  s.description = %(== Riff
Riff is a wrapper around diff that highlights not only which lines have changed,
but also which parts of the lines that have changed.

= Usage

$ git diff | riff

Or if you do...

$ git config --global pager.diff riff

$ git config --global pager.show riff

... then all future 'git diff's and 'git show's will be
refined.
)
  s.authors     = ['Johan Walles']
  s.email       = 'johan.walles@gmail.com'
  s.homepage    = 'http://github.com/walles/riff'
  s.license     = 'MIT'

  s.files         = `git ls-files`.split("\n")
  s.test_files    = `git ls-files -- {test,spec,features}/*`.split("\n")
  s.executables   = ['riff']
  s.require_paths = ['lib']

  # Development is done on 2.0, and we're using at least __dir__ which requires
  # Ruby 2.0.
  s.required_ruby_version = '~> 2.0'

  s.add_development_dependency 'rspec', '~> 3.0'
  s.add_development_dependency 'rubocop', '~ 0.29.1'

  s.add_runtime_dependency 'diff-lcs', '~> 1.2.5'
  s.add_runtime_dependency 'slop', '~> 4.1.0'
end
