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
      expect(highlighted).to end_with(
        "#{RED}-last line#{reversed('↵')}\n" \
        "#{GREEN}+last line#{RESET}\n")
    end
  end
end
