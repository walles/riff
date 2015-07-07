require 'whitespace_lint'

include WhitespaceLint

RSpec.describe WhitespaceLint, '#collect_ws_highlights' do
  context 'empty string' do
    it 'returns the empty set' do
      expect(collect_ws_highlights('')).to be_empty
    end
  end
end
