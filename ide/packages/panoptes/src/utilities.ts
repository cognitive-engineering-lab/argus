import { SerializedTree, TreeTopology } from '@argus/common/types';

interface Path<T, Direction> {
    from: T,
    to: T,
    path: T[],
}

type Direction = 'ToRoot' | 'FromRoot';

export function toRoot(tree: SerializedTree, from: number): Path<number, 'ToRoot'> {
    let root = tree.descr.root;
    let topo = tree.topology;
    let path = [from];
    let current = from;
    while (current !== root) {
        let parent = topo.parent[current];
        path.push(parent);
        current = parent;
    }

    return {
        from, to: root, path
    };
}

export function fromRoot(tree: SerializedTree, from: number): Path<number, 'FromRoot'> {
    let { from: f, to, path } = toRoot(tree, from);
    return {
        from: to, to: f, path: path.reverse()
    };
}