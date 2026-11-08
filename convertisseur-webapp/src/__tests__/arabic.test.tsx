import { render, screen } from "@testing-library/react";
import { Arabic } from "@/components/Arabic";

describe("Arabic", () => {
	it("marks the text as right-to-left Arabic", () => {
		render(<Arabic>بسم الله</Arabic>);

		const text = screen.getByText("بسم الله");
		expect(text).toHaveAttribute("lang", "ar");
		expect(text).toHaveAttribute("dir", "rtl");
	});
});
