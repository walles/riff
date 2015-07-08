require 'whitespace_lint'

include WhitespaceLint

RSpec.describe WhitespaceLint, '#collect_ws_highlights' do
  context 'empty string' do
    it 'returns an empty set' do
      expect(collect_ws_highlights('')).to be_empty
    end
  end

  context 'a line that is fine' do
    it 'returns an empty set' do
      expect(collect_ws_highlights("\t text\n")).to be_empty
    end
  end

  context 'line with only a space' do
    it 'highlights the space' do
      expect(collect_ws_highlights(" \n")).to eq(Set.new [0])
    end
  end

  context 'line with only a tab' do
    it 'highlights the tab' do
      expect(collect_ws_highlights("\t\n")).to eq(Set.new [0])
    end
  end

  context 'line with text + space' do
    it 'highlights the space' do
      expect(collect_ws_highlights("012 \n")).to eq(Set.new [3])
    end
  end

  context 'two lines, second ending in space' do
    string = "012\n345 \n"

    it 'highlights the space' do
      highlights = collect_ws_highlights(string)
      expect(highlights.size).to eq(1)

      highlight = highlights.to_a()[0]
      expect(string[highlight]).to eq(' ')
    end
  end
end
