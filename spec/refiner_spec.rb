require 'colors'
require 'refiner'

include Colors

RSpec.describe Refiner, '#print_diff' do
  context 'with two single character strings' do
    it 'fully highlights both the added and the removed sections' do
      refiner = Refiner.new('x', 'y')
      expect(refiner.refined_old.to_s).to eq "#{RED}-#{REVERSE}x#{RESET}"
      expect(refiner.refined_new.to_s).to eq "#{GREEN}+#{REVERSE}y#{RESET}"
    end
  end
end
