# Rule Matching via Ordered Lists

We choose to implement rule lookup via ordered lists (not hash tables or tree strucutres) because
- they need no pointers,
- can easily be split into pages,
- have low complexity in search code,
- inconsistencies lead to wrong results in the worst case

Ordered lists require that all rules can be sorted linearly without any ambiguity.

The user facing configuration should allow the following features:
- match executable paths with wildcards in the path or all executables
- match entire domains, multiple domains, multiple hosts or even any remote host
- match IP address ranges
- define a list or range of ports to match
- define a list of protocols to match

It is obvious that these rules have overlapping matches and a precedence must be defined in order
to resolve the ambiguity.

Most rule properties define ranges, e.g. a domain is a range of host names, blocklists define
address ranges etc. An absolute order for binary search is only possible if ranges used by
different rules don't overlap. We therefore divide all ranges into non-overlapping subranges.
For each subrange, the highest precedence rule is used.

We use binary search for executables, IP addresses and host names, but not for ports, protocols,
directions etc. These properties are searched sequentially once the proper list is found.
