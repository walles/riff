# coding: utf-8
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
        "#{GREEN}+#{reversed('1')}2#{reversed('3')}#{RESET}\n")
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
        "#{GREEN}+#{reversed('1')}234#{reversed('5')}#{RESET}\n")
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
        "#{GREEN}+a\n" \
        "#{GREEN}+b#{RESET}\n")
    end
  end

  context %(with added newline) do
    diff_string = DiffString.new('+', GREEN)
    diff_string.add('a', false)
    diff_string.add('b', false)
    diff_string.add("\n", true)
    diff_string.add('c', false)
    diff_string.add('d', false)
    diff_string.add("\n", false)

    it %(properly highlights the newline) do
      expect(diff_string.to_s).to eq(
        %(#{GREEN}+ab#{reversed('↵')}\n) +
        %(#{GREEN}+cd#{RESET}\n))
    end
  end
end
