# Design

(algorithm-goals)=
## Goals

The core reduction algorithm has three goals:

1. Go fast
2. Produce small test cases
3. Produce readable test cases

Unlike more theoretical work in this space, the algorithm does *not* attempt to
minimize the number of "oracle queries", that is, invocations of the
user-provided "interestingness test".

(algorithm-assumptions)=
## Assumptions

These assumptions strongly inform the algorithm design:

1. The interestingness will be comparatively slow---it will generally involve
   spinning up a new process, disk I/O, parsing, type-checking, etc.
2. Smaller inputs lead to faster interestingness tests.

These assumptions hold for the use-case of reducing programs that cause compiler
crashes.

## High-Level Design

Due to [Assumption (1)](algorithm-assumptions), it's essential that `treedd`
execute several interestingness tests in parallel. Luckily, for the same reason,
lock contention is unlikely to be a major issue---so long as executing the
interestingness test doesn't require holding a lock, most threads will spend the
majority of their time executing the interestingness test, rather than waiting
for locks on shared data.

The recent paper "[PARDIS][pardis] : Priority Aware Test Case Reduction"
highlights the importance of *prioritization* of reductions. Greedy removal of
the *largest* subtrees leads to greatly increased performance due to faster
interestingness tests ([Assumption (2)](algorithm-assumptions)).

With these ideas in mind, the overall algorithm design involves spinning up some
number of threads, which share two pieces of mutable (locked) data: the target
program being minimized, and a prioritized max-heap of reductions to attempt.
Each thread executes the following loop:

- Pop a reduction task off the heap
- Execute the interestingness test with the reduced program
- If the reduced program was still interesting, try replacing the global target:

  * If the target was replaced by another thread, try this reduction again
  * Otherwise, replace the target with the reduced version

- Push any new tasks onto the task queue

If lock contention does become an issue, it may be beneficial for each thread to
maintain a local prioritized heap in addition to the global one, or even to
reduce a local copy of the tree multiple times before attempting to replace the
global copy.

## Reduction Strategies

`treedd` uses several strategies during program minimization:

- *Deletion* (TODO([#1][#1])): When a child is optional, `treedd` attempts to
  delete it. For example, `treedd` might delete the `const` in `const int x;`.
- *Delta debugging* (TODO([#2][#2])): When a node has a list of children,
  `treedd` uses *delta debugging* to delete as many as possible in an efficient
  way.
- *Hoisting* (TODO([#3][#3])): Nodes with a recursive structure may be replaced
  by their descendants, e.g. replacing `5 + (3 * y)` with just `y`.

## Pseudocode

A few notes:

- Because the tree may be replaced by any thread at any time, tasks store *node
  IDs*, rather than nodes themselves, and have to check if those nodes are still
  in the current tree when executing a task.
- In practice, `weight` is simply the number of bytes in the source text of the
  node.

```python
class NodeId:
    ...

class Node:
    ...  # defined in tree-sitter

    def is_list(self) -> bool:
        ...

    def is_optional(self) -> bool:
        ...

class Heap:
    ...

class Tree:
    def find(self, node_id: NodeId) -> Node:
        ...

    def render(self) -> str:
        ...

    def replace(self, old_node, new_node) -> Tree:
        ...

    def root_node(self) -> Node:
        ...

enum Task:
    Explore(NodeId)
    Delete(NodeId)
    Hoist(NodeId, NodeId)
    Delta(NodeId)

class PrioritizedTask:
    task: Task
    priority: int

def treedd(source_code: str) -> str:
    tree = parse(source_code)
    root = tree.root_node()
    heap = Heap()
    heap.push(PrioritizedTask(Explore(NodeId(root)), priority=weight(root)))
    threads = AtomicUsize(0)
    idle_threads = AtomicUsize(0)
    fork(spawn, tree, heap, threads, idle_threads)

    # Wait for all threads to finish and exit:
    while tree.count_references() > 1:
        wait()

    return tree.extract().render()

# -----------------------------------------------------------
# Parallel structure of the computation

def spawn(tree: Tree, heap: Heap, threads: AtomicUsize, idle_threads: AtomicUsize) -> None:
    threads += 1
    idle = False
    while True:
        if idle:
            idle_threads -= 1
            idle = False
        heap.lock()
        match heap.pop_max():
            case None:
                heap.unlock()
                idle = idle_logic(idle_threads)
            case task:
                heap.unlock()
                dispatch(tree, heap, task)

def idle_logic(idle_threads: AtomicUSize) -> None:
    idle_threads += 1
    if idle_threads == NUM_THREADS:
        exit_thread()
    sleep()  # some kind of backoff
    return True

# -----------------------------------------------------------
# Reduction logic

def dispatch(tree: Tree, heap: Heap, task: Task) -> None:
    match task:
        case Explore(node_id):
            explore(tree, heap, node_id)
        case Delete(node_id):
            delete(tree, heap, node_id)
        case Hoist(node_id, node_id):
            assert False, "Unimplemented" # TODO(lb)
        case Delta(node_id):
            assert False, "Unimplemented" # TODO(lb)

def explore(tree: Tree, heap: Heap, node_id: NodeId) -> None:
    with tree.read_lock():
        node = tree.find(node_id)
        with heap.lock():
            if node.is_optional():
                heap.push(PrioritizedTask(Delete(node, priority=weight(node))))
            else:
                for child in node.children():
                    heap.push(PrioritizedTask(Explore(child, priority=weight(child))))
            # TODO(lb): Other tasks

def delete(tree: Tree, heap: Heap, node_id: NodeId) -> None:
    with tree.read_lock():
        node = tree.find(node_id)
        with heap.lock():
            if node.is_optional():
                heap.push(PrioritizedTask(Delete(node, priority=weight(node))))
            else:
                for child in node.children():
                    heap.push(PrioritizedTask(Explore(child, priority=weight(child))))
            # TODO(lb): Other tasks

# -----------------------------------------------------------
# Helpers

def interesting_replacement(tree, node, variant):
    return interesting(tree.replace(node, variant).render())

def weight(node):
    ...

def parse(source_code: str) -> Tree:
    ...

def exit_thread():
    ...
```

(bib)=
## Bibliography

TODO(#16): BibTeX

- Gharachorlu, G. and Sumner, N., 2019, April. : Priority Aware Test Case
  Reduction. In International Conference on Fundamental Approaches to Software
  Engineering (pp. 409-426). Springer, Cham.
- Sun, C., Li, Y., Zhang, Q., Gu, T. and Su, Z., 2018, May. Perses:
  Syntax-guided program reduction. In Proceedings of the 40th International
  Conference on Software Engineering (pp. 361-371).
- Hodován, R. and Kiss, Á., 2016, July. Practical Improvements to the Minimizing
  Delta Debugging Algorithm. In ICSOFT-EA (pp. 241-248).
- Hodován, R. and Kiss, Á., 2016, November. Modernizing hierarchical delta
  debugging. In Proceedings of the 7th International Workshop on Automating Test
  Case Design, Selection, and Evaluation (pp. 31-37).
- Vince, D., Hodován, R., Bársony, D. and Kiss, Á., 2021, May. Extending
  Hierarchical Delta Debugging with Hoisting. In 2021 IEEE/ACM International
  Conference on Automation of Software Test (AST) (pp. 60-69). IEEE.
- Kiss, Á., Hodován, R. and Gyimóthy, T., 2018, November. HDDr: a recursive
  variant of the hierarchical delta debugging algorithm. In Proceedings of the
  9th ACM SIGSOFT International Workshop on Automating TEST Case Design,
  Selection, and Evaluation (pp. 16-22).
- Hodován, R., Kiss, Á. and Gyimóthy, T., 2017, September. Coarse hierarchical
  delta debugging. In 2017 IEEE international conference on software maintenance
  and evolution (ICSME) (pp. 194-203). IEEE.

[#1]: https://github.com/langston-barrett/treedd/issues/1
[#2]: https://github.com/langston-barrett/treedd/issues/2
[#3]: https://github.com/langston-barrett/treedd/issues/3
[#16]: https://github.com/langston-barrett/treedd/issues/16
[pardis]: https://github.com/golnazgh/PARDIS