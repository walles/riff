require 'rspec/core/rake_task'

task default: :spec
desc 'Run the unit tests (default)'
task spec: [:deps]
RSpec::Core::RakeTask.new do |t|
  t.pattern = FileList['spec/**/*_spec.rb']
end

desc 'Create a .gem package'
task package: [:spec] do
  system('rm -f *.gem ; gem build riffdiff.gemspec') || fail
end

desc 'Install dependencies'
task :deps do
  system('bundle install') || fail
end

desc 'Publish to rubygems.org'
task publish: [:package] do
  system('gem push *.gem') || fail
end
