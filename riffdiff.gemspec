require 'English'

git_version = begin
  describe = `git describe --always`.strip
  fail unless $CHILD_STATUS.success?
  fail unless /^([0-9]+([.][0-9]+)+)(-([0-9]+)-[^-]+)?$/.match(describe)
  $4 ? "#{$1}.#{$4}" : "#{$1}.0"
end

Gem::Specification.new do |s|
  s.name        = 'riffdiff'
  s.version     = git_version
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
