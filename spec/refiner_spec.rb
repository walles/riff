require 'colors'
require 'refiner'

include Colors

RSpec.describe Refiner, '#new' do
  context 'with "x"->"y"' do
    it 'fully highlights both "x" and "y"' do
      refiner = Refiner.new('x', 'y')
      expect(refiner.refined_old.to_s).to eq "#{RED}-#{REVERSE}x#{RESET}"
      expect(refiner.refined_new.to_s).to eq "#{GREEN}+#{REVERSE}y#{RESET}"
    end
  end

  context 'with single quotes to double quotes' do
    it 'highlights the quotes and nothing else' do
      refiner = Refiner.new("'quoted'", '"quoted"')
      expect(refiner.refined_old.to_s).to eq(
        %(#{RED}-#{REVERSE}'#{NORMAL}quoted#{REVERSE}'#{RESET}))
      expect(refiner.refined_new.to_s).to eq(
        %(#{GREEN}+#{REVERSE}"#{NORMAL}quoted#{REVERSE}"#{RESET}))
    end
  end
end
