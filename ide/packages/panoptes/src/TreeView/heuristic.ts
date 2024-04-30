import { ProofNodeIdx } from "@argus/common/bindings";
import _ from "lodash";

import { isTraitClause, mean, stdDev } from "../utilities/func";
import TreeInfo from "./TreeInfo";

interface HeuristicI {
  // Partition sets of failed nodes into two groups.
  partition(
    groups: _.Dictionary<ProofNodeIdx[]>
  ): [ProofNodeIdx[], ProofNodeIdx[]];

  rank<T>(data: T[], f: (t: T) => ProofNodeIdx): T[];
}

export function treeHeuristic(tree: TreeInfo): HeuristicI {
  return new Heuristic(tree);
}

type Strategy = {
  size: number;
  sorter: (group: ProofNodeIdx[]) => number;
  partitioner: (n: number) => <T>(groups: T[]) => [T[], T[]];
};

type PartitionTup<T> = [T[], T[]];
function takeN(n: number) {
  return function <T>(groups: T[]): PartitionTup<T> {
    return [_.slice(groups, 0, n), _.slice(groups, n)];
  };
}

function takeAll<T>(groups: T[]) {
  return takeN(groups.length)(groups);
}

class Heuristic implements HeuristicI {
  public constructor(readonly tree: TreeInfo) {}

  numMainUninferred(group: ProofNodeIdx[]) {
    return _.reduce(
      group,
      (acc, leaf) =>
        acc +
        (() => {
          const node = this.tree.node(leaf);
          if ("Goal" in node) {
            const goal = this.tree.goal(node.Goal);
            return goal.isMainTv ? 1 : 0;
          } else {
            return 0;
          }
        })(),
      0
    );
  }

  numPrincipled(group: ProofNodeIdx[]) {
    return group.length - this.numMainUninferred(group);
  }

  maxDepth(group: ProofNodeIdx[]) {
    return _.max(_.map(group, leaf => this.tree.depth(leaf)))!;
  }

  numOutliers<T>(group: T[], f: (g: T) => number) {
    const data = _.map(group, f);
    const meanD = mean(data);
    const stdDevD = stdDev(data, meanD);
    return _.filter(data, d => d > meanD + stdDevD).length;
  }

  sortByNPT() {
    return (group: ProofNodeIdx[]) => -this.numPrincipled(group);
  }

  sortByNPTRatio() {
    return (group: ProofNodeIdx[]) => group.length / this.numPrincipled(group);
  }

  sortByDepth() {
    return (group: ProofNodeIdx[]) =>
      -_.max(_.map(group, leaf => this.tree.depth(leaf)))!;
  }

  // ----------------------
  // Partitioning Algorithm

  // NOTE: partitioning with heuristics can alsways be improved.
  partition(
    failedGroups: _.Dictionary<ProofNodeIdx[]>
  ): PartitionTup<ProofNodeIdx> {
    // Getting the right group is important but it's not precise.
    // We currently use the following metrics.
    //
    // 1. Depth of the group. Generally, deep obligations are "more interesting."
    //
    // 2. The number of obligations whose "principle types" are known (NPT).
    //    What this entails is that in an obligation such as `TYPE: TRAIT`,
    //    neither TYPE nor TRAIT is an unresolved type variable `_`.
    //
    //    We use this number, NPT, to find groups whose NPT is more than
    //    one standard deviation from the mean. This is useful when trait impls are
    //    macro generated for varying arities, larger arities have a high number of
    //    unresolved type variables.
    //
    // 3. The NPT is useful, until it isn't. We use the ratio of group size by
    //    NPT to favor smaller groups with more concrete types.
    const failedValues = _.values(failedGroups);
    const strategies: Strategy[] = [
      {
        size: this.numOutliers(failedValues, g => this.maxDepth(g)),
        sorter: this.sortByDepth(),
        partitioner: takeN,
      },
      {
        size: this.numOutliers(failedValues, g => this.numPrincipled(g)),
        sorter: this.sortByNPT(),
        partitioner: takeN,
      },
      {
        size: this.numOutliers(
          failedValues,
          group => group.length / this.numPrincipled(group)
        ),
        sorter: this.sortByNPTRatio(),
        partitioner: takeN,
      },
    ];

    const sortedAndPartitioned = _.map(strategies, s =>
      this.applyStrategy(failedValues, s)
    );

    // Combine all strategies to get the final partition.
    const [importantGroupsNonUnq, restNonUnq] = _.reduce(
      sortedAndPartitioned,
      ([accImp, accRest], [imp, rest]) => [
        _.concat(accImp, _.flatten(imp)),
        _.concat(accRest, _.flatten(rest)),
      ],
      [[], []] as PartitionTup<ProofNodeIdx>
    );

    const [importantGroups, restWithDups] = [
      _.uniq(importantGroupsNonUnq),
      _.uniq(restNonUnq),
    ];

    // Remove recommended nodes from the hidden nodes, this can happen if
    // one sort strategy recommends one while another doesn't.
    const rest = _.difference(restWithDups, importantGroups);

    // The "Argus recommended" errors are shown expanded, and the
    // "others" are collapsed. Argus recommended errors are the ones
    // that failed or are ambiguous with a concrete type on the LHS.
    const [argusRecommendedLeaves, others] =
      this.partitionSingles(importantGroups);

    // Fallback to sorting by depth and showing everything. This, of course, is a HACK.
    if (argusRecommendedLeaves.length === 0) {
      return this.partitionDefault(failedValues);
    }

    const hiddenLeaves = _.concat(rest, others);
    return [argusRecommendedLeaves, hiddenLeaves];
  }

  applyStrategy(
    groups: ProofNodeIdx[][],
    { size, sorter, partitioner }: Strategy
  ) {
    return size > 0
      ? partitioner(size)(_.sortBy(groups, sorter))
      : ([[], []] as PartitionTup<ProofNodeIdx>);
  }

  partitionSingles(group: ProofNodeIdx[]) {
    return _.partition(group, leaf => {
      const node = this.tree.node(leaf);
      if ("Goal" in node) {
        const goal = this.tree.goal(node.Goal);
        const result = this.tree.result(goal.result);
        return (
          !goal.isMainTv && (result === "no" || result === "maybe-overflow")
        );
      } else {
        // Leaves should only be goals...
        throw new Error(`Leaves should only be goals ${node}`);
      }
    });
  }

  partitionDefault(failedGroups: ProofNodeIdx[][]) {
    const defaultStrategy = {
      size: failedGroups.length,
      sorter: this.sortByDepth(),
      partitioner: (_: any) => takeAll,
    };
    const [important, hiddenLeaves] = this.applyStrategy(
      failedGroups,
      defaultStrategy
    );
    const [argusRecommendedLeaves, others] = this.partitionSingles(
      _.flatten(important)
    );
    return [
      argusRecommendedLeaves,
      _.concat(_.flatten(hiddenLeaves), others),
    ] as PartitionTup<ProofNodeIdx>;
  }

  // -------------------------
  // Sort a list of flat nodes

  rank<T>(data: T[], f: (t: T) => ProofNodeIdx) {
    const sortErrorsFirst = (t: T) => {
      const leaf = f(t);
      switch (this.tree.nodeResult(leaf)) {
        case "no":
          return 0;
        case "maybe-overflow":
        case "maybe-ambiguity":
          return 1;
        case "yes":
          return 2;
      }
    };

    const sortWeightPaths = (t: T) => {
      const leaf = f(t);
      const pathToRoot = this.tree.pathToRoot(leaf);
      const len = pathToRoot.path.length;
      const numVars = _.reduce(
        pathToRoot.path,
        (sum, k) => sum + this.tree.inferVars(k),
        0
      );

      return numVars / len;
    };

    const bubbleTraitClauses = (t: T) => {
      const leaf = f(t);
      const n = this.tree.node(leaf);
      if (
        "Goal" in n &&
        isTraitClause(this.tree.goal(n.Goal).value.predicate)
      ) {
        return 0;
      }
      return 1;
    };

    const recommendedOrder = _.sortBy(data, [
      sortErrorsFirst,
      bubbleTraitClauses,
      sortWeightPaths,
    ]);

    return recommendedOrder;
  }

  errorLeavesInSimpleRecommendedOrder() {
    return this.rank(this.tree.errorLeaves(), _.identity);
  }
}
