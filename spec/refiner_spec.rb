require 'colors'
require 'refiner'

include Colors

def reversed(string)
  return "#{REVERSE}#{string}#{NOT_REVERSE}"
end

RSpec.describe Refiner, '#new' do
  context 'with "x"->"y"' do
    refiner = Refiner.new("x\n", "y\n")

    it 'fully highlights both "x" and "y"' do
      expect(refiner.refined_old.to_s).to eq(
        "#{RED}-" + reversed('x') + "#{RESET}\n")
      expect(refiner.refined_new.to_s).to eq(
        "#{GREEN}+" + reversed('y') + "#{RESET}\n")
    end
  end

  context 'with single quotes to double quotes' do
    refiner = Refiner.new(%('quoted'\n), %("quoted"\n))

    it 'highlights the quotes and nothing else' do
      expect(refiner.refined_old.to_s).to eq(
        %(#{RED}-#{reversed("'")}quoted#{reversed("'")}#{RESET}\n))
      expect(refiner.refined_new.to_s).to eq(
        %(#{GREEN}+#{reversed('"')}quoted#{reversed('"')}#{RESET}\n))
    end
  end

  context 'with empty old' do
    refiner = Refiner.new('', "something\n")

    it 'refines old to the empty string' do
      expect(refiner.refined_old.to_s).to eq('')
    end

    it 'does not highlight anything in new' do
      expect(refiner.refined_new.to_s).to eq(
        %(#{GREEN}+something#{RESET}\n))
    end
  end

  context 'with empty new' do
    refiner = Refiner.new("something\n", '')

    it 'does not highlight anything in old' do
      expect(refiner.refined_old.to_s).to eq(
        %(#{RED}-something#{RESET}\n))
    end

    it 'refines new to the empty string' do
      expect(refiner.refined_new.to_s).to eq('')
    end
  end

  context %(with <x "hej"> to <x 'hej'>) do
    refiner = Refiner.new(%(x "hej"\n), %(x 'hej'\n))

    it 'highlights the quotes and nothing else' do
      expect(refiner.refined_old.to_s).to eq(
        %(#{RED}-x #{reversed('"')}hej#{reversed('"')}#{RESET}\n))
      expect(refiner.refined_new.to_s).to eq(
        %(#{GREEN}+x #{reversed("'")}hej#{reversed("'")}#{RESET}\n))
    end
  end
end
