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
      diff_string.add("\n", false)
      expect(diff_string.to_s).to eq(
        "#{GREEN}+#{REVERSE}1#{NOT_REVERSE}2#{REVERSE}3#{NOT_REVERSE}#{RESET}\n")
    end

    it 'renders correctly with a few chars in between' do
      diff_string = DiffString.new('+', GREEN)
      diff_string.add('1', true)
      diff_string.add('2', false)
      diff_string.add('3', false)
      diff_string.add('4', false)
      diff_string.add('5', true)
      diff_string.add("\n", false)
      expect(diff_string.to_s).to eq(
        "#{GREEN}+#{REVERSE}1#{NOT_REVERSE}234#{REVERSE}5#{NOT_REVERSE}#{RESET}\n")
    end
  end

  context 'multiline' do
    it 'colors both lines' do
      diff_string = DiffString.new('+', GREEN)
      diff_string.add('a', false)
      diff_string.add("\n", false)
      diff_string.add('b', false)
      diff_string.add("\n", false)

      expect(diff_string.to_s).to eq(
        "#{GREEN}+a\n" +
        "#{GREEN}+b#{RESET}\n")
    end
  end
end
