import A

extension U: Comparable {
    func <(lhs: Self, rhs: Self) -> Bool {
        switch (lhs, rhs) {
            case (.x, .y):
                return true
            default:
                return false
        }
    }
}

public func insert(e: U, es: Set<U>) -> Void {
    es.insert(e)
    return
}
