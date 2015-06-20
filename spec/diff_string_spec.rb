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

  context 'single char special color, on green' do
    diff_string = DiffString.new('+', GREEN)
    diff_string.add('1', false)
    diff_string.add('2', true, RED)
    diff_string.add('3', false)
    diff_string.add("\n", false)

    it 'renders correctly' do
      expect(diff_string.to_s).to eq(
        "#{GREEN}+1#{reversed("#{RED}2")}#{GREEN}3#{RESET}\n")
    end
  end

  context 'double char special color, on green' do
    diff_string = DiffString.new('+', GREEN)
    diff_string.add('1', false)
    diff_string.add('2', true, RED)
    diff_string.add('x', true, RED)
    diff_string.add('3', false)
    diff_string.add("\n", false)

    it 'renders correctly' do
      expect(diff_string.to_s).to eq(
        "#{GREEN}+1#{reversed("#{RED}2x")}#{GREEN}3#{RESET}\n")
    end
  end

  context 'special color at start, on green' do
    diff_string = DiffString.new('+', GREEN)
    diff_string.add('1', true, RED)
    diff_string.add('2', false)
    diff_string.add('3', false)
    diff_string.add("\n", false)

    it 'renders correctly' do
      expect(diff_string.to_s).to eq(
        "#{GREEN}+#{reversed("#{RED}1")}#{GREEN}23#{RESET}\n")
    end
  end

  context 'special color at end, on green' do
    diff_string = DiffString.new('+', GREEN)
    diff_string.add('1', false)
    diff_string.add('2', false)
    diff_string.add('3', true, RED)
    diff_string.add("\n", false)

    it 'renders correctly' do
      expect(diff_string.to_s).to eq(
        "#{GREEN}+12#{reversed("#{RED}3")}#{RESET}\n")
    end
  end

  context 'special color before reverse, on green' do
    diff_string = DiffString.new('+', GREEN)
    diff_string.add('1', false)
    diff_string.add('2', true, RED)
    diff_string.add('3', true)
    diff_string.add("\n", false)

    it 'renders correctly' do
      expect(diff_string.to_s).to eq(
        "#{GREEN}+1#{reversed("#{RED}2#{GREEN}3")}#{RESET}\n")
    end
  end

  context 'special color after reverse, on green' do
    diff_string = DiffString.new('+', GREEN)
    diff_string.add('1', false)
    diff_string.add('2', true)
    diff_string.add('3', true, RED)
    diff_string.add("\n", false)

    it 'renders correctly' do
      expect(diff_string.to_s).to eq(
        "#{GREEN}+1#{reversed("2#{RED}3")}#{RESET}\n")
    end
  end

  context 'special color at end of first line, on green' do
    diff_string = DiffString.new('+', GREEN)
    diff_string.add('1', false)
    diff_string.add('2', true, RED)
    diff_string.add("\n", false)
    diff_string.add('3', false)
    diff_string.add("\n", false)

    it 'renders correctly' do
      expect(diff_string.to_s).to eq(
        "#{GREEN}+1#{reversed("#{RED}2")}#{GREEN}\n3#{RESET}\n")
    end
  end

  context 'special color before reversed newline, on green' do
    diff_string = DiffString.new('+', GREEN)
    diff_string.add('1', false)
    diff_string.add('2', true, RED)
    diff_string.add("\n", true)
    diff_string.add('3', false)
    diff_string.add("\n", false)

    it 'renders correctly' do
      expect(diff_string.to_s).to eq(
        "#{GREEN}+1#{reversed("#{RED}2#{GREEN}↵")}\n3#{RESET}\n")
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

  context %(with highlighted ending newline) do
    diff_string = DiffString.new('+', GREEN)
    diff_string.add('x', false)
    diff_string.add('y', false)
    diff_string.add("\n", true)

    it %(properly highlights the newline) do
      expect(diff_string.to_s).to eq(
        %(#{GREEN}+xy#{reversed('↵')}#{RESET}\n))
    end
  end

  context %(empty) do
    diff_string = DiffString.new('+', GREEN)

    it %(is empty) do
      expect(diff_string.to_s).to eq('')
    end
  end

  context %(doesn't end in a newline) do
    diff_string = DiffString.new('+', GREEN)
    diff_string.add('x', false)

    it %(gets a newline added) do
      expect(diff_string.to_s).to eq(
        %(#{GREEN}+x#{RESET}\n))
    end
  end
end
