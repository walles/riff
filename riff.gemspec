Gem::Specification.new do |s|
  s.name        = 'riff'
  s.version     = '0.0.0'
  s.date        = system('git log -1 --format=%cd --date=short | cat')
  s.summary     = 'A diff highlighter showing intra line differences'
  s.authors     = ['Johan Walles']
  s.email       = 'johan.walles@gmail.com'
  s.files       = ['lib/riff.rb']
  s.homepage    = 'http://github.com/walles/riff'
  s.license     = 'MIT'
end
