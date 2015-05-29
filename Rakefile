require 'rspec/core/rake_task'

task default: :spec
desc 'Run the unit tests (default)'
RSpec::Core::RakeTask.new do |t|
  t.pattern = FileList['spec/**/*_spec.rb']
end

desc 'Create a .gem package'
task package: [:spec] do
  system('rm -f *.gem ; gem build riffdiff.gemspec')
end
