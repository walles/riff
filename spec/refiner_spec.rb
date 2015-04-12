require 'colors'
require 'refiner'

include Colors

RSpec.describe Refiner, '#new' do
  context 'with "x"->"y"' do
    it 'fully highlights both "x" and "y"' do
      refiner = Refiner.new("x\n", "y\n")
      expect(refiner.refined_old.to_s).to eq(
        "#{RED}-#{REVERSE}x#{NOT_REVERSE}#{RESET}\n")
      expect(refiner.refined_new.to_s).to eq(
        "#{GREEN}+#{REVERSE}y#{NOT_REVERSE}#{RESET}\n")
    end
  end

  context 'with single quotes to double quotes' do
    it 'highlights the quotes and nothing else' do
      refiner = Refiner.new(%('quoted'\n), %("quoted"\n))
      expect(refiner.refined_old.to_s).to eq(
        %(#{RED}-#{REVERSE}'#{NOT_REVERSE}quoted#{REVERSE}'#{NOT_REVERSE}#{RESET}\n))
      expect(refiner.refined_new.to_s).to eq(
        %(#{GREEN}+#{REVERSE}"#{NOT_REVERSE}quoted#{REVERSE}"#{NOT_REVERSE}#{RESET}\n))
    end
  end
end
