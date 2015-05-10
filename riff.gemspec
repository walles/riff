Gem::Specification.new do |s|
  s.name        = 'riff'
  s.version     = `git describe --dirty`
  s.summary     = 'A diff highlighter showing intra line differences'
  s.authors     = ['Johan Walles']
  s.email       = 'johan.walles@gmail.com'
  s.homepage    = 'http://github.com/walles/riff'
  s.license     = 'MIT'

  s.files         = `git ls-files`.split("\n")
  s.test_files    = `git ls-files -- {test,spec,features}/*`.split("\n")
  s.executables   = ['riff']
  s.require_paths = ['lib']
end
