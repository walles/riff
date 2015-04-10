require 'colors'
require 'diff_string'

include Colors

RSpec.describe DiffString, '#add' do
  context 'first and last highlighted' do
    it 'renders correctly with one char in between' do
      diff_string = DiffString.new('+', GREEN)
      diff_string.add('1', true)
      diff_string.add('2', false)
      diff_string.add('3', true)
      expect(diff_string.to_s).to eq(
        "#{GREEN}+#{REVERSE}1#{NORMAL}2#{REVERSE}3#{RESET}")
    end

    it 'renders correctly with a few chars in between' do
      diff_string = DiffString.new('+', GREEN)
      diff_string.add('1', true)
      diff_string.add('2', false)
      diff_string.add('3', false)
      diff_string.add('4', false)
      diff_string.add('5', true)
      expect(diff_string.to_s).to eq(
        "#{GREEN}+#{REVERSE}1#{NORMAL}234#{REVERSE}5#{RESET}")
    end
  end

  context 'multiline' do
    it 'colors both lines' do
      diff_string = DiffString.new('+', GREEN)
      diff_string.add('a', false)
      diff_string.add("\n", false)
      diff_string.add('b', false)

      expect(diff_string.to_s).to eq(
        "#{GREEN}+a\n" +
        "#{GREEN}+b#{RESET}")
    end
  end
end
