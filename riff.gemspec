Gem::Specification.new do |s|
  s.name        = 'riff'
  s.version     = '0.0.0'
  s.summary     = 'A diff highlighter showing intra line differences'
  s.authors     = ['Johan Walles']
  s.email       = 'johan.walles@gmail.com'
  s.homepage    = 'http://github.com/walles/riff'
  s.license     = 'MIT'

  s.files         = `git ls-files`.split("\n")
  s.test_files    = `git ls-files -- {test,spec,features}/*`.split("\n")
  s.executables   = `git ls-files -- bin/*`.split("\n").map { |f|
    File.basename(f)
  }
  s.require_paths = ['lib']
end
