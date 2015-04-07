require 'refiner'

RSpec.describe Refiner, '#print_diff' do
  context 'with two single character strings' do
    it 'fully highlights both the added and the removed sections' do
      refiner = Refiner.new('x', 'y')
      expect(refiner.refined_old.to_s).to eq 'gris'
      expect(refiner.refined_new.to_s).to eq 'flaska'
    end
  end
end
