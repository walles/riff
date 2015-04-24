# coding: utf-8
require 'colors'

include Colors

RSpec.describe Colors, '#uncolor' do
  context 'string with no colors' do
    not_colored = 'not colored'

    it 'does not change the contents' do
      expect(uncolor(not_colored)).to eq(not_colored)
    end
  end

  context 'all our codes' do
    [BOLD, CYAN, GREEN, RED, REVERSE, NOT_REVERSE, RESET].each do |code|
      printable_code = code.gsub(ESC, '\e')
      it "can remove #{printable_code} by itself" do
        expect(uncolor(code)).to eq('')
      end

      it "can remove #{printable_code} from the start of the string" do
        expect(uncolor("#{code}foo")).to eq('foo')
      end

      it "can remove #{printable_code} from the end of the string" do
        expect(uncolor("bar#{code}")).to eq('bar')
      end

      it "can remove #{printable_code} from the middle of the string" do
        expect(uncolor("hej#{code}san")).to eq('hejsan')
      end
    end
  end
end
