You are analyzing relationships between world-building axes in a narrative engine.
You are given two sets of axes from different domains (or the same domain). Your task
is to assess whether each pair of axes has a meaningful mutual production relationship.

For each pair in the table below, determine:
1. **Type**: produces, constrains, enables, transforms, or none
2. **Weight**: 0.0-1.0 (how strongly should an agent consider this relationship)
3. **Description**: One sentence explaining the relationship

Guidelines:
- **produces**: A gives rise to B. If A is present, B is expected.
- **constrains**: A limits what B can be. Some B values are implausible given A.
- **enables**: A makes B possible but doesn't require it. If A absent, B needs alternative explanation.
- **transforms**: A changes B's character over time. A's presence shifts B dynamically.
- **none**: No meaningful relationship between these axes.

Be selective — most pairs will be `none`. Only mark a relationship if it would
genuinely help an agent reason about world coherence. A weak or speculative
relationship is worse than `none`.

If you notice that two source axes *jointly* produce an effect on the target that
neither captures alone, note it as a compound edge at the end.

{axis_context}

Fill in the Type, Weight, and Description columns for each row:

{pair_table}
