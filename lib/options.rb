require 'slop'

# Handle command line options
module Options
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
      o.on '-h', '--help', 'Print this help' do
        puts o
        exit
      end
    end

    if $stdin.isatty
      puts opts
      exit
    end

    begin
      opts.parse(ARGV)
    rescue Slop::Error => e
      STDERR.puts "ERROR: #{e}"
      STDERR.puts
      STDERR.puts opts
      exit 1
    end
  end
end
