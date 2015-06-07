# coding: utf-8
require 'colors'
require 'refiner'

include Colors

RSpec.describe Refiner, '#new' do
  context 'with single quotes to double quotes' do
    refiner = Refiner.new(%('quoted'\n), %("quoted"\n))

    it 'highlights the quotes and nothing else' do
      expect(refiner.refined_old).to eq(
        %(#{RED}-#{reversed("'")}quoted#{reversed("'")}#{RESET}\n))
      expect(refiner.refined_new).to eq(
        %(#{GREEN}+#{reversed('"')}quoted#{reversed('"')}#{RESET}\n))
    end
  end

  context 'with empty old' do
    refiner = Refiner.new('', "something\n")

    it 'refines old to the empty string' do
      expect(refiner.refined_old).to eq('')
    end

    it 'does not highlight anything in new' do
      expect(refiner.refined_new).to eq(
        %(#{GREEN}+something#{RESET}\n))
    end
  end

  context 'with empty new' do
    refiner = Refiner.new("something\n", '')

    it 'does not highlight anything in old' do
      expect(refiner.refined_old).to eq(
        %(#{RED}-something#{RESET}\n))
    end

    it 'refines new to the empty string' do
      expect(refiner.refined_new).to eq('')
    end
  end

  context %(with <x "hej"> to <x 'hej'>) do
    refiner = Refiner.new(%(x "hej"\n), %(x 'hej'\n))

    it 'highlights the quotes and nothing else' do
      expect(refiner.refined_old).to eq(
        %(#{RED}-x #{reversed('"')}hej#{reversed('"')}#{RESET}\n))
      expect(refiner.refined_new).to eq(
        %(#{GREEN}+x #{reversed("'")}hej#{reversed("'")}#{RESET}\n))
    end
  end

  context %(with too many differences) do
    refiner = Refiner.new("0123456789\n",
                          "abcdefghij\n")

    it %(doesn't highlight anything) do
      expect(refiner.refined_old).to eq(
        %(#{RED}-0123456789#{RESET}\n))
      expect(refiner.refined_new).to eq(
        %(#{GREEN}+abcdefghij#{RESET}\n))
    end
  end

  context %(with added ending newline) do
    refiner = Refiner.new('abcde',
                          "abcde\n")

    it %(highlights the newline) do
      expect(refiner.refined_old).to eq(
        %(#{RED}-abcde#{RESET}\n))
      expect(refiner.refined_new).to eq(
        %(#{GREEN}+abcde#{reversed('↵')}#{RESET}\n))
    end

    it %(ends in a newline) do
      expect(refiner.refined_old).to end_with("\n")
      expect(refiner.refined_new).to end_with("\n")
    end
  end

  context %(with removed ending newline) do
    refiner = Refiner.new("abcde\n",
                          'abcde')

    it %(highlights the newline) do
      expect(refiner.refined_old).to eq(
        %(#{RED}-abcde#{reversed('↵')}#{RESET}\n))
      expect(refiner.refined_new).to eq(
        %(#{GREEN}+abcde#{RESET}\n))
    end

    it %(ends in a newline) do
      expect(refiner.refined_old).to end_with("\n")
      expect(refiner.refined_new).to end_with("\n")
    end
  end

  context %(with one line turning into many) do
    refiner = Refiner.new("abcde\n",
                          "abcde,\n" \
                          "fffff,\n" \
                          "ggggg\n")

    it %(pretends first line is unchanged, but highlights comma) do
      expect(refiner.refined_old).to eq('')
      expect(refiner.refined_new).to eq(
        %( abcde#{reversed(',')}\n) +
        %(#{GREEN}+fffff,\n) +
        %(#{GREEN}+ggggg#{RESET}\n))
    end
  end

  context %(with two lines turning into many) do
    refiner = Refiner.new("abcde\n" \
                          "fffff\n",
                          "abcde,\n" \
                          "fffff,\n" \
                          "ggggg\n")

    it %(highlights the comma on the first two lines, but not the extra line) do
      expect(refiner.refined_old).to eq(
        %(#{RED}-abcde\n) +
        %(#{RED}-fffff#{RESET}\n))
      expect(refiner.refined_new).to eq(
        %(#{GREEN}+abcde#{reversed(',')}\n) +
        %(#{GREEN}+fffff#{reversed(',')}\n) +
        %(#{GREEN}+ggggg#{RESET}\n))
    end
  end

  context %(with one line being replaced with many) do
    refiner = Refiner.new("abcdx\n",
                          "abcde,\n" \
                          "fffff,\n" \
                          "ggggg\n")

    it %(highlights both adds and removes on the first line) do
      expect(refiner.refined_old).to eq(
        %(#{RED}-abcd#{reversed('x')}#{RESET}\n))
      expect(refiner.refined_new).to eq(
        %(#{GREEN}+abcd#{reversed('e,')}\n) +
        %(#{GREEN}+fffff,\n) +
        %(#{GREEN}+ggggg#{RESET}\n))
    end
  end

  context %(with many lines turning into one) do
    refiner = Refiner.new("abcde,\n" \
                          "fffff,\n" \
                          "ggggg\n",
                          "abcde\n")

    it %(highlights the first removed comma, but not the two removed lines) do
      expect(refiner.refined_old).to eq(
        %(#{RED}-abcde#{reversed(',')}\n) +
        %(#{RED}-fffff,\n) +
        %(#{RED}-ggggg#{RESET}\n))
      expect(refiner.refined_new).to eq(
        %(#{GREEN}+abcde#{RESET}\n))
    end
  end

  context %(with large input) do
    # A Refiner that fails if trying to collect highlights
    class NonHighlightingRefiner < Refiner
      def collect_highlights(_diff, _old_highlights, _new_highlights)
        fail "Shouldn't collect highlights"
      end
    end

    old = "0123456789\n"
    new = '0123456789' * 1500 + "\n"

    it %(doesn't even attempt highlighting) do
      NonHighlightingRefiner.new(old, new)
    end
  end
end
