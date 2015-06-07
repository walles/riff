require 'slop'
require 'version'

# Handle command line options
module Options
  include Version

  def handle_options
    opts = Slop::Options.new do |o|
      o.banner = <<-EOS
Usage: diff ... | riff
Colors diff and highlights what parts of changed lines have changed.

Git integration:
    git config --global pager.diff riff
    git config --global pager.show riff
EOS
      o.separator 'Options:'
      o.on '--version', 'Print version information and exit' do
        puts "riff #{version}"
        puts
        puts 'Developed at http://github.com/walles/riff'
        puts
        puts 'Get the source code:'
        puts '  git clone git@github.com:walles/riff.git'

        exit
      end
      o.on '-h', '--help', 'Print this help' do
        puts o
        exit
      end
    end

    begin
      opts.parse(ARGV)
    rescue Slop::Error => e
      STDERR.puts "ERROR: #{e}"
      STDERR.puts
      STDERR.puts opts
      exit 1
    end

    if $stdin.isatty
      puts opts
      exit
    end
  end
end
