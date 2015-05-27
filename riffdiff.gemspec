$LOAD_PATH.unshift File.join(File.absolute_path(__dir__), 'lib')
require 'version'

Gem::Specification.new do |s|
  include Version

  s.name        = 'riffdiff'
  s.version     = semantify_git_version(git_version())
  s.summary     = 'A diff highlighter showing what parts of lines have changed'
  s.authors     = ['Johan Walles']
  s.email       = 'johan.walles@gmail.com'
  s.homepage    = 'http://github.com/walles/riff'
  s.license     = 'MIT'

  s.files         = `git ls-files`.split("\n")
  s.test_files    = `git ls-files -- {test,spec,features}/*`.split("\n")
  s.executables   = ['riff']
  s.require_paths = ['lib']
end
