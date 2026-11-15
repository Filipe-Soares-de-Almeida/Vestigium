import { expect, test } from "vitest";

import { show } from "../dev/test-mount";
import Spark from "./Spark.svelte";

const props = { label: "Seconds", from: "first", to: "last", floor: 0, ceiling: 100 };

/** The drawn stretches of the line: each starts with an M. */
const stretches = (page: HTMLElement) => (page.querySelector("path.stroke")?.getAttribute("d")?.match(/M/g) ?? []).length;

test("a_reading_that_was_not_taken_breaks_the_line_instead_of_dropping_to_zero", () => {
  const page = show(Spark, { ...props, values: [10, 20, null, 30, 40] });

  expect(stretches(page)).toBe(2);
});

test("a_reading_between_two_gaps_is_drawn_as_a_point", () => {
  const page = show(Spark, { ...props, values: [10, null, 30, null, 50, 60] });

  expect(stretches(page)).toBe(1);
  expect(page.querySelectorAll(".lone")).toHaveLength(2);
});
