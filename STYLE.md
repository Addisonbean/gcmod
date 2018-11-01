* Use Option::map and Result::map for mapping/transforming values only, not for side effects. Keep it functional. (example of what to not do, then replace it with an `if let ...` + an example of what to do)
* Anytime you use the unreachable macro, add a comment explaining why it's unreachable (unless it's pretty obvious, use discretion)
* Tabs
