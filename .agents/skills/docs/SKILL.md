---
name: Writing Documentation in Markdown
description: This skill handles the creation of markdown documents to document the system.
---

# Skill: Writing Documentation in Markdown

## Domain Context

This skill handles the creation of markdown documents to document the system.

## Technical Constraints

1. Markdown files are usually stored in `docs/` (unless requested otherwise) but may be placed inside of subdirectories.
2. Markdown files are properly formatted using a structured document format (so favoring headers to represent document structure).
3. Write at a 10th—12th grade level.
4. Assume a knowledge base of a person with a bachelors-level understanding of of theoretical computer science and the equivalent of a minor in linguistics
5. Limit line length to 100 characters.

## Specific Guidance

* Prefer that pseudocode be written in mathematical pseudocode. If that is not possible, then use rust.
* Files should always be UTF-8
* Files should have frontmatter with their `name`, `description`, and `target_audience` included as fields. 
