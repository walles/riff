# coding: utf-8
require 'colors'
require 'riff'

include Colors

RSpec.describe Riff, '#handle_diff_line' do
  context 'Removed newline at end of file' do
    highlighted =
      Riff.new.do_stream(
        File.open(File.join(__dir__, 'removed-newline-at-eof.diff')))

    it 'ends the right way' do
      expect(highlighted.split("\n", -1)[-3..-1]).to eq(
        "#{RED}-  needing to set color.diff=false.#{reversed('↵')}\n" \
        "#{GREEN}+  needing to set color.diff=false.#{RESET}\n".split("\n", -1))
    end
  end
end
